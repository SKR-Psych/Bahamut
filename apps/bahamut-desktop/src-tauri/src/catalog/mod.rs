//! Versioned model catalogue. The manifest (models.json, embedded at compile
//! time) is data, not code: adding, disabling, or re-tiering models means
//! editing JSON, never application logic. Validation runs once at load and
//! again in tests so a malformed manifest fails fast.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const TIERS: [&str; 3] = ["lightweight", "balanced", "high-performance"];
pub const CATEGORIES: [&str; 3] = ["coding", "reasoning", "general"];
const CAPABILITIES: [&str; 3] = ["basic", "good", "strong"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub display_name: String,
    pub ollama_tag: String,
    pub family: String,
    pub category: String,
    pub license: String,
    pub license_flagged: bool,
    pub source: String,
    pub download_size_gb: f64,
    pub min_ram_gb: f64,
    pub recommended_ram_gb: f64,
    pub min_vram_gb: f64,
    pub recommended_vram_gb: f64,
    pub context_length: u64,
    pub tool_use: bool,
    pub code_generation: String,
    pub reasoning: String,
    pub tier: String,
    pub strengths: String,
    pub limitations: String,
    pub notes: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCatalog {
    pub version: u32,
    pub updated: String,
    pub models: Vec<CatalogModel>,
}

const MANIFEST: &str = include_str!("models.json");
static CATALOG: OnceLock<Result<ModelCatalog, String>> = OnceLock::new();

pub fn load_catalog() -> Result<&'static ModelCatalog, String> {
    CATALOG
        .get_or_init(|| {
            let catalog: ModelCatalog = serde_json::from_str(MANIFEST)
                .map_err(|e| format!("Model catalogue is malformed: {}", e))?;
            validate(&catalog)?;
            Ok(catalog)
        })
        .as_ref()
        .map_err(|e| e.clone())
}

pub fn validate(catalog: &ModelCatalog) -> Result<(), String> {
    if catalog.version == 0 {
        return Err("Catalogue version must be >= 1".to_string());
    }
    let mut ids = std::collections::HashSet::new();
    let mut tags = std::collections::HashSet::new();
    for model in &catalog.models {
        let ctx = |msg: &str| format!("Catalogue entry '{}': {}", model.id, msg);
        if !ids.insert(model.id.clone()) {
            return Err(ctx("duplicate id"));
        }
        if !tags.insert(model.ollama_tag.clone()) {
            return Err(ctx("duplicate ollama_tag"));
        }
        if !TIERS.contains(&model.tier.as_str()) {
            return Err(ctx(&format!("unknown tier '{}'", model.tier)));
        }
        if !CATEGORIES.contains(&model.category.as_str()) {
            return Err(ctx(&format!("unknown category '{}'", model.category)));
        }
        for (label, value) in [
            ("code_generation", &model.code_generation),
            ("reasoning", &model.reasoning),
        ] {
            if !CAPABILITIES.contains(&value.as_str()) {
                return Err(ctx(&format!("invalid {} rating '{}'", label, value)));
            }
        }
        if model.license.trim().is_empty() {
            return Err(ctx("license must not be empty"));
        }
        if model.min_ram_gb > model.recommended_ram_gb
            || model.min_vram_gb > model.recommended_vram_gb
        {
            return Err(ctx("minimum requirements exceed recommended requirements"));
        }
        if model.download_size_gb <= 0.0 {
            return Err(ctx("download size must be positive"));
        }
        // Models with flagged licences must explain themselves.
        if model.license_flagged && model.notes.trim().is_empty() {
            return Err(ctx("license_flagged entries require explanatory notes"));
        }
    }
    for tier in TIERS {
        if !catalog
            .models
            .iter()
            .any(|m| m.enabled && m.tier == tier)
        {
            return Err(format!("Tier '{}' has no enabled models", tier));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Hardware-tier recommendation
// ---------------------------------------------------------------------------

/// Maps detected hardware to a recommendation tier. VRAM dominates (GPU
/// inference is the practical bottleneck); undetected VRAM falls back to a
/// RAM-only judgement so the wizard still works on machines where GPU
/// detection failed.
pub fn hardware_tier(total_ram_gb: f64, vram_gb: Option<f64>) -> &'static str {
    let vram = vram_gb.unwrap_or(0.0);
    if vram >= 12.0 || total_ram_gb >= 32.0 {
        "high-performance"
    } else if vram >= 6.0 || total_ram_gb >= 16.0 {
        "balanced"
    } else {
        "lightweight"
    }
}

#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub tier: String,
    /// Primary pick: the first enabled coding model in the tier (the
    /// catalogue's order within a tier expresses preference).
    pub primary_id: Option<String>,
    /// Every enabled model the machine meets the minimum requirements for.
    pub compatible_ids: Vec<String>,
}

pub fn recommend(
    catalog: &ModelCatalog,
    total_ram_gb: f64,
    vram_gb: Option<f64>,
) -> Recommendation {
    let tier = hardware_tier(total_ram_gb, vram_gb);
    let vram = vram_gb.unwrap_or(0.0);

    let primary_id = catalog
        .models
        .iter()
        .filter(|m| m.enabled && m.tier == tier)
        .filter(|m| total_ram_gb >= m.min_ram_gb && vram >= m.min_vram_gb)
        .min_by_key(|m| if m.category == "coding" { 0 } else { 1 })
        .map(|m| m.id.clone())
        // If nothing in the computed tier fits (e.g. low-RAM edge), fall back
        // to the most capable model the machine can actually run.
        .or_else(|| {
            catalog
                .models
                .iter()
                .filter(|m| m.enabled && total_ram_gb >= m.min_ram_gb && vram >= m.min_vram_gb)
                .max_by(|a, b| {
                    a.recommended_ram_gb
                        .partial_cmp(&b.recommended_ram_gb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|m| m.id.clone())
        });

    let compatible_ids = catalog
        .models
        .iter()
        .filter(|m| m.enabled && total_ram_gb >= m.min_ram_gb && vram >= m.min_vram_gb)
        .map(|m| m.id.clone())
        .collect();

    Recommendation {
        tier: tier.to_string(),
        primary_id,
        compatible_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_valid() {
        let catalog = load_catalog().expect("embedded catalogue must validate");
        assert!(catalog.version >= 1);
        assert!(catalog.models.len() >= 3);
    }

    #[test]
    fn validation_rejects_duplicates_and_bad_fields() {
        let mut catalog = load_catalog().unwrap().clone();
        let dup = catalog.models[0].clone();
        catalog.models.push(dup);
        assert!(validate(&catalog).unwrap_err().contains("duplicate id"));

        let mut catalog = load_catalog().unwrap().clone();
        catalog.models[0].tier = "ultra".to_string();
        assert!(validate(&catalog).unwrap_err().contains("unknown tier"));

        let mut catalog = load_catalog().unwrap().clone();
        catalog.models[0].license = " ".to_string();
        assert!(validate(&catalog).unwrap_err().contains("license"));

        let mut catalog = load_catalog().unwrap().clone();
        catalog.models[0].min_ram_gb = 99.0;
        assert!(validate(&catalog)
            .unwrap_err()
            .contains("exceed recommended"));
    }

    #[test]
    fn every_tier_has_an_enabled_model() {
        let catalog = load_catalog().unwrap();
        for tier in TIERS {
            assert!(
                catalog.models.iter().any(|m| m.enabled && m.tier == tier),
                "tier {} empty",
                tier
            );
        }
    }

    #[test]
    fn flagged_licenses_are_marked_and_explained() {
        let catalog = load_catalog().unwrap();
        let llama = catalog
            .models
            .iter()
            .find(|m| m.id == "llama3.2-3b")
            .unwrap();
        assert!(llama.license_flagged);
        assert!(!llama.enabled, "flagged license stays disabled until review");
        assert!(llama.notes.to_lowercase().contains("license"));
    }

    #[test]
    fn hardware_tiers_map_sensibly() {
        assert_eq!(hardware_tier(8.0, None), "lightweight");
        assert_eq!(hardware_tier(8.0, Some(4.0)), "lightweight");
        assert_eq!(hardware_tier(16.0, None), "balanced");
        assert_eq!(hardware_tier(8.0, Some(8.0)), "balanced");
        assert_eq!(hardware_tier(32.0, None), "high-performance");
        assert_eq!(hardware_tier(16.0, Some(16.0)), "high-performance");
    }

    #[test]
    fn recommendation_prefers_coding_models_and_respects_minimums() {
        let catalog = load_catalog().unwrap();

        // Typical modern PC: balanced tier, Qwen2.5 Coder 7B primary.
        let rec = recommend(catalog, 16.0, Some(8.0));
        assert_eq!(rec.tier, "balanced");
        assert_eq!(rec.primary_id.as_deref(), Some("qwen2.5-coder-7b"));
        assert!(rec.compatible_ids.contains(&"qwen2.5-coder-1.5b".to_string()));

        // Low-spec laptop: lightweight coding pick.
        let rec = recommend(catalog, 8.0, None);
        assert_eq!(rec.tier, "lightweight");
        assert_eq!(rec.primary_id.as_deref(), Some("qwen2.5-coder-1.5b"));
        // High-performance models are not offered as compatible.
        assert!(!rec.compatible_ids.contains(&"qwen2.5-coder-32b".to_string()));

        // Workstation: high-performance tier.
        let rec = recommend(catalog, 64.0, Some(24.0));
        assert_eq!(rec.tier, "high-performance");
        assert_eq!(rec.primary_id.as_deref(), Some("qwen2.5-coder-14b"));
        assert!(rec.compatible_ids.contains(&"qwen2.5-coder-32b".to_string()));

        // Disabled (license-flagged) models never appear.
        let rec = recommend(catalog, 16.0, Some(8.0));
        assert!(!rec.compatible_ids.contains(&"llama3.2-3b".to_string()));
    }
}
