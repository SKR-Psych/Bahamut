use crate::catalog::{
    self, HardwareProfile, HardwareTier, ModelCatalogueEntry, ModelRecommendation,
};
use crate::commands::settings::AiSettings;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderStatus {
    pub provider: String,
    pub reachable: bool,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledModel {
    pub id: String,
    pub display_name: String,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<String>,
    pub digest: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    pub model: String,
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub done: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub conversation_id: Option<i64>,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub conversation_id: Option<i64>,
    pub content: String,
    pub done: bool,
    pub error: Option<String>,
}

pub fn validate_ollama_endpoint(endpoint: &str) -> Result<String, String> {
    let url =
        Url::parse(endpoint).map_err(|_| "Ollama endpoint must be a valid URL".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Ollama endpoint must use http:// or https://".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Ollama endpoint must include a host".to_string())?;
    let safe = matches!(host, "localhost" | "127.0.0.1" | "::1") || host.starts_with("127.");
    if !safe {
        return Err("Only loopback Ollama endpoints are allowed in this milestone; remote endpoints require a future security decision.".to_string());
    }
    Ok(endpoint.trim_end_matches('/').to_string())
}

fn client(timeout_ms: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms.clamp(1_000, 300_000)))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

#[derive(Deserialize)]
struct VersionResp {
    version: Option<String>,
}
#[derive(Deserialize)]
struct TagsResp {
    models: Vec<OllamaModel>,
}
#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    size: Option<u64>,
    modified_at: Option<String>,
    digest: Option<String>,
    details: Option<serde_json::Value>,
}

pub async fn provider_status(settings: &AiSettings) -> Result<ProviderStatus, String> {
    let endpoint = validate_ollama_endpoint(&settings.ollama_endpoint)?;
    let resp = client(settings.request_timeout_ms)?
        .get(format!("{endpoint}/api/version"))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => {
            let version = r.json::<VersionResp>().await.ok().and_then(|v| v.version);
            Ok(ProviderStatus {
                provider: "ollama".into(),
                reachable: true,
                version,
                message: "Ollama is reachable".into(),
            })
        }
        Ok(r) => Ok(ProviderStatus {
            provider: "ollama".into(),
            reachable: false,
            version: None,
            message: format!("Ollama returned HTTP {}", r.status()),
        }),
        Err(e) if e.is_timeout() => Ok(ProviderStatus {
            provider: "ollama".into(),
            reachable: false,
            version: None,
            message: "Ollama request timed out".into(),
        }),
        Err(e) => Ok(ProviderStatus {
            provider: "ollama".into(),
            reachable: false,
            version: None,
            message: format!("Ollama is unreachable: {e}"),
        }),
    }
}

pub async fn installed_models(settings: &AiSettings) -> Result<Vec<InstalledModel>, String> {
    let endpoint = validate_ollama_endpoint(&settings.ollama_endpoint)?;
    let r = client(settings.request_timeout_ms)?
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .map_err(|e| format!("Failed to list models: {e}"))?;
    if !r.status().is_success() {
        return Err(format!(
            "Ollama returned HTTP {} while listing models",
            r.status()
        ));
    }
    let tags = r
        .json::<TagsResp>()
        .await
        .map_err(|e| format!("Malformed Ollama model list: {e}"))?;
    Ok(tags
        .models
        .into_iter()
        .map(|m| InstalledModel {
            id: m.name.clone(),
            display_name: m.name,
            size_bytes: m.size,
            modified_at: m.modified_at,
            digest: m.digest,
            details: m.details,
        })
        .collect())
}

pub async fn delete_model(settings: &AiSettings, model: &str) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("Model id is required".into());
    }
    let endpoint = validate_ollama_endpoint(&settings.ollama_endpoint)?;
    let r = client(settings.request_timeout_ms)?
        .delete(format!("{endpoint}/api/delete"))
        .json(&serde_json::json!({"model": model}))
        .send()
        .await
        .map_err(|e| format!("Failed to delete model: {e}"))?;
    if r.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Ollama returned HTTP {} while deleting model",
            r.status()
        ))
    }
}

fn parse_progress_line(model: &str, line: &str) -> Result<PullProgress, String> {
    let v: serde_json::Value =
        serde_json::from_str(line).map_err(|e| format!("Malformed pull progress: {e}"))?;
    Ok(PullProgress {
        model: model.into(),
        status: v
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("working")
            .into(),
        completed: v.get("completed").and_then(|n| n.as_u64()),
        total: v.get("total").and_then(|n| n.as_u64()),
        done: v.get("done").and_then(|n| n.as_bool()).unwrap_or(false),
    })
}

pub async fn pull_model(
    app: AppHandle,
    settings: AiSettings,
    model: String,
    generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
) -> Result<(), String> {
    let endpoint = validate_ollama_endpoint(&settings.ollama_endpoint)?;
    let my_generation = generation.load(Ordering::SeqCst);
    let r = client(settings.request_timeout_ms)?
        .post(format!("{endpoint}/api/pull"))
        .json(&serde_json::json!({"model": model, "stream": true}))
        .send()
        .await
        .map_err(|e| format!("Failed to start pull: {e}"))?;
    if !r.status().is_success() {
        return Err(format!(
            "Ollama returned HTTP {} while pulling model",
            r.status()
        ));
    }
    let text = r
        .text()
        .await
        .map_err(|e| format!("Failed to read pull stream: {e}"))?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if generation.load(Ordering::SeqCst) != my_generation {
            let _ = app.emit("local-ai://pull-cancelled", &model);
            return Ok(());
        }
        let progress = parse_progress_line(&model, line)?;
        let _ = app.emit("local-ai://pull-progress", &progress);
    }
    Ok(())
}

pub async fn chat(
    app: AppHandle,
    settings: AiSettings,
    req: ChatRequest,
    generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
) -> Result<String, String> {
    let endpoint = validate_ollama_endpoint(&settings.ollama_endpoint)?;
    let my_generation = generation.load(Ordering::SeqCst);
    let r = client(settings.request_timeout_ms)?.post(format!("{endpoint}/api/chat")).json(&serde_json::json!({"model": req.model, "messages": req.messages, "stream": true, "options": {"temperature": req.temperature.unwrap_or(settings.temperature), "num_predict": req.max_output_tokens.unwrap_or(settings.max_output_tokens)}})).send().await.map_err(|e| format!("Failed to start chat: {e}"))?;
    if !r.status().is_success() {
        return Err(format!(
            "Ollama returned HTTP {} while chatting",
            r.status()
        ));
    }
    let mut full = String::new();
    let text = r
        .text()
        .await
        .map_err(|e| format!("Failed to read chat stream: {e}"))?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if generation.load(Ordering::SeqCst) != my_generation {
            let _ = app.emit("local-ai://chat-cancelled", req.conversation_id);
            return Ok(full);
        }
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("Malformed chat response: {e}"))?;
        let content = v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        if !content.is_empty() {
            full.push_str(&content);
        }
        let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
        let _ = app.emit(
            "local-ai://chat-chunk",
            &ChatChunk {
                conversation_id: req.conversation_id,
                content,
                done,
                error: None,
            },
        );
    }
    Ok(full)
}

pub fn catalogue() -> Vec<ModelCatalogueEntry> {
    catalog::built_in_catalogue()
}
pub fn recommend(profile: &HardwareProfile) -> Vec<ModelRecommendation> {
    catalog::recommendations(profile)
}
pub fn hardware_profile() -> HardwareProfile {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let mut p = HardwareProfile {
        total_ram_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        cpu_cores: sys.cpus().len(),
        gpu_model: detect_gpu_name(),
        vram_gb: detect_vram_gb(),
        tier: HardwareTier::UnknownGpu,
        detection_notes: vec![],
    };
    if p.vram_gb.is_none() {
        p.detection_notes.push(
            "VRAM detection is unavailable on this platform or returned no adapter memory.".into(),
        );
    }
    p.tier = catalog::tier_for(&p);
    p
}
#[cfg(target_os = "windows")]
fn detect_gpu_name() -> Option<String> {
    std::process::Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name", "/value"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Name=").map(|v| v.trim().to_string()))
                .filter(|v| !v.is_empty())
        })
}
#[cfg(not(target_os = "windows"))]
fn detect_gpu_name() -> Option<String> {
    None
}
#[cfg(target_os = "windows")]
fn detect_vram_gb() -> Option<f64> {
    std::process::Command::new("wmic")
        .args([
            "path",
            "win32_VideoController",
            "get",
            "AdapterRAM",
            "/value",
        ])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("AdapterRAM=")
                    .and_then(|v| v.trim().parse::<u64>().ok())
            })
        })
        .map(|b| b as f64 / 1024.0 / 1024.0 / 1024.0)
}
#[cfg(not(target_os = "windows"))]
fn detect_vram_gb() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_validation_rejects_remote() {
        assert!(validate_ollama_endpoint("http://example.com:11434").is_err());
        assert!(validate_ollama_endpoint("http://127.0.0.1:11434").is_ok());
    }
    #[test]
    fn pull_progress_parses() {
        let p =
            parse_progress_line("m", r#"{"status":"pulling","completed":5,"total":10}"#).unwrap();
        assert_eq!(p.completed, Some(5));
    }
}
