//! Model-provider abstraction. The UI and command layer speak only these
//! provider-neutral types; `ollama` is the first concrete provider. Future
//! providers (other OpenAI-compatible local servers, llama.cpp runtimes,
//! cloud providers) implement the same shapes behind new modules without
//! touching the frontend.

pub mod ollama;

use serde::{Deserialize, Serialize};

/// Provider reachability, refreshed on demand — reconnecting never requires
/// an application restart.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub endpoint: String,
    pub reachable: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InstalledModel {
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
}

/// Extra metadata for one installed model (best effort).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelDetails {
    pub context_length: Option<u64>,
    pub family: Option<String>,
    pub license_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationOptions {
    pub temperature: f64,
    pub max_output_tokens: u64,
    pub timeout_secs: u64,
}

/// Streamed model-pull progress (sent to the frontend over a channel).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PullProgress {
    pub status: String,
    pub total_bytes: Option<u64>,
    pub completed_bytes: Option<u64>,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PullOutcome {
    Completed,
    Cancelled,
}

/// Streamed chat event (sent to the frontend over a channel).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChatStreamEvent {
    pub token: Option<String>,
    pub done: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatOutcome {
    Completed,
    Cancelled,
}
