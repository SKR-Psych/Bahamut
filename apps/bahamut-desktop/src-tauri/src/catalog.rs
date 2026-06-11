use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCatalogueEntry {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub family: String,
    pub parameters_b: f32,
    pub quantization: String,
    pub context_window: u32,
    pub download_size_gb: f32,
    pub min_ram_gb: f32,
    pub recommended_ram_gb: f32,
    pub min_vram_gb: Option<f32>,
    pub recommended_vram_gb: Option<f32>,
    pub license: String,
    pub license_url: String,
    pub safety_notes: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub total_ram_gb: f64,
    pub cpu_cores: usize,
    pub gpu_model: Option<String>,
    pub vram_gb: Option<f64>,
    pub tier: HardwareTier,
    pub detection_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    Low,
    Balanced,
    Performance,
    UnknownGpu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRecommendation {
    pub model: ModelCatalogueEntry,
    pub fit: RecommendationFit,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationFit {
    Recommended,
    Compatible,
    Caution,
}

pub fn built_in_catalogue() -> Vec<ModelCatalogueEntry> {
    vec![
        entry(EntrySpec {
            id: "qwen2.5-coder:1.5b",
            display_name: "Qwen2.5 Coder 1.5B",
            parameters_b: 1.5,
            min_ram_gb: 4.0,
            recommended_ram_gb: 8.0,
            min_vram_gb: None,
            recommended_vram_gb: Some(2.0),
            download_size_gb: 1.0,
            license: "Apache-2.0",
            license_url: "https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct",
        }),
        entry(EntrySpec {
            id: "qwen2.5-coder:3b",
            display_name: "Qwen2.5 Coder 3B",
            parameters_b: 3.0,
            min_ram_gb: 8.0,
            recommended_ram_gb: 12.0,
            min_vram_gb: Some(3.0),
            recommended_vram_gb: Some(4.0),
            download_size_gb: 2.0,
            license: "Apache-2.0",
            license_url: "https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct",
        }),
        entry(EntrySpec {
            id: "qwen2.5-coder:7b",
            display_name: "Qwen2.5 Coder 7B",
            parameters_b: 7.0,
            min_ram_gb: 16.0,
            recommended_ram_gb: 24.0,
            min_vram_gb: Some(6.0),
            recommended_vram_gb: Some(8.0),
            download_size_gb: 4.7,
            license: "Apache-2.0",
            license_url: "https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct",
        }),
        entry(EntrySpec {
            id: "llama3.2:3b",
            display_name: "Llama 3.2 3B Instruct",
            parameters_b: 3.0,
            min_ram_gb: 8.0,
            recommended_ram_gb: 16.0,
            min_vram_gb: Some(4.0),
            recommended_vram_gb: Some(6.0),
            download_size_gb: 2.0,
            license: "Llama 3.2 Community License",
            license_url: "https://www.llama.com/llama3_2/license/",
        }),
        entry(EntrySpec {
            id: "mistral:7b",
            display_name: "Mistral 7B Instruct",
            parameters_b: 7.0,
            min_ram_gb: 16.0,
            recommended_ram_gb: 24.0,
            min_vram_gb: Some(6.0),
            recommended_vram_gb: Some(8.0),
            download_size_gb: 4.1,
            license: "Apache-2.0",
            license_url: "https://huggingface.co/mistralai/Mistral-7B-Instruct-v0.3",
        }),
    ]
}

struct EntrySpec<'a> {
    id: &'a str,
    display_name: &'a str,
    parameters_b: f32,
    min_ram_gb: f32,
    recommended_ram_gb: f32,
    min_vram_gb: Option<f32>,
    recommended_vram_gb: Option<f32>,
    download_size_gb: f32,
    license: &'a str,
    license_url: &'a str,
}

fn entry(spec: EntrySpec<'_>) -> ModelCatalogueEntry {
    ModelCatalogueEntry { id: spec.id.to_string(), provider: "ollama".to_string(), display_name: spec.display_name.to_string(), family: spec.display_name.split_whitespace().next().unwrap_or("local").to_string(), parameters_b: spec.parameters_b, quantization: "provider-default".to_string(), context_window: 32_768, download_size_gb: spec.download_size_gb, min_ram_gb: spec.min_ram_gb, recommended_ram_gb: spec.recommended_ram_gb, min_vram_gb: spec.min_vram_gb, recommended_vram_gb: spec.recommended_vram_gb, license: spec.license.to_string(), license_url: spec.license_url.to_string(), safety_notes: "Local model; verify upstream licence and acceptable-use terms before redistribution or commercial deployment.".to_string(), tags: vec!["local".to_string(), "chat".to_string(), "coding".to_string()] }
}

pub fn tier_for(profile: &HardwareProfile) -> HardwareTier {
    if profile.vram_gb.is_none() {
        return HardwareTier::UnknownGpu;
    }
    let ram = profile.total_ram_gb;
    let vram = profile.vram_gb.unwrap_or(0.0);
    if ram >= 32.0 && vram >= 10.0 {
        HardwareTier::Performance
    } else if ram >= 16.0 && vram >= 6.0 {
        HardwareTier::Balanced
    } else {
        HardwareTier::Low
    }
}

pub fn recommendations(profile: &HardwareProfile) -> Vec<ModelRecommendation> {
    let mut out: Vec<ModelRecommendation> = built_in_catalogue()
        .into_iter()
        .map(|m| {
            let ram_ok = profile.total_ram_gb as f32 >= m.min_ram_gb;
            let vram_ok = match (profile.vram_gb, m.min_vram_gb) {
                (Some(v), Some(min)) => v as f32 >= min,
                (None, Some(_)) => false,
                _ => true,
            };
            let recommended = profile.total_ram_gb as f32 >= m.recommended_ram_gb
                && match (profile.vram_gb, m.recommended_vram_gb) {
                    (Some(v), Some(min)) => v as f32 >= min,
                    (None, Some(_)) => false,
                    _ => true,
                };
            let fit = if recommended {
                RecommendationFit::Recommended
            } else if ram_ok && vram_ok {
                RecommendationFit::Compatible
            } else {
                RecommendationFit::Caution
            };
            let mut reasons = vec![format!("Requires at least {:.0} GB RAM", m.min_ram_gb)];
            if let Some(v) = m.min_vram_gb {
                reasons.push(format!("Best with at least {:.0} GB VRAM", v));
            }
            let mut warnings = Vec::new();
            if !ram_ok {
                warnings.push("System RAM is below the listed minimum.".to_string());
            }
            if profile.vram_gb.is_none() {
                warnings.push(
                    "VRAM could not be detected; recommendation is conservative.".to_string(),
                );
            } else if !vram_ok {
                warnings.push("Detected VRAM is below the listed minimum.".to_string());
            }
            ModelRecommendation {
                model: m,
                fit,
                reasons,
                warnings,
            }
        })
        .collect();
    out.sort_by_key(|r| match r.fit {
        RecommendationFit::Recommended => 0,
        RecommendationFit::Compatible => 1,
        RecommendationFit::Caution => 2,
    });
    out
}

pub fn validate_catalogue(entries: &[ModelCatalogueEntry]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for e in entries {
        if e.id.trim().is_empty() || e.provider.trim().is_empty() || e.license.trim().is_empty() {
            return Err("Catalogue entries must include id, provider and license".into());
        }
        if !ids.insert(e.id.as_str()) {
            return Err(format!("Duplicate model id {}", e.id));
        }
        if e.min_ram_gb <= 0.0 || e.recommended_ram_gb < e.min_ram_gb {
            return Err(format!("Invalid RAM bounds for {}", e.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_validates() {
        validate_catalogue(&built_in_catalogue()).unwrap();
    }
    #[test]
    fn unknown_vram_fallback_is_conservative() {
        let p = HardwareProfile {
            total_ram_gb: 16.0,
            cpu_cores: 8,
            gpu_model: None,
            vram_gb: None,
            tier: HardwareTier::UnknownGpu,
            detection_notes: vec![],
        };
        assert!(recommendations(&p).iter().any(|r| !r.warnings.is_empty()));
    }
    #[test]
    fn recommendation_tiers() {
        let mut p = HardwareProfile {
            total_ram_gb: 32.0,
            cpu_cores: 12,
            gpu_model: Some("GPU".into()),
            vram_gb: Some(12.0),
            tier: HardwareTier::Performance,
            detection_notes: vec![],
        };
        p.tier = tier_for(&p);
        assert_eq!(p.tier, HardwareTier::Performance);
        assert!(recommendations(&p)
            .iter()
            .any(|r| matches!(r.fit, RecommendationFit::Recommended)));
    }
}
