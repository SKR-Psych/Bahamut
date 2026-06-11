//! Ollama HTTP client. All network behaviour lives here; the JSON-line
//! parsers are pure functions so streaming logic is testable without a
//! running Ollama. Endpoints used: /api/version, /api/tags, /api/show,
//! /api/pull, /api/chat, /api/delete.

use super::{
    ChatMessage, ChatOutcome, ChatStreamEvent, GenerationOptions, InstalledModel, ModelDetails,
    ProviderStatus, PullOutcome, PullProgress,
};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

fn client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Client without a total-request timeout (streams run long); connect
/// timeout still applies.
fn streaming_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

// ---------------------------------------------------------------------------
// Pure parsers (unit-tested without a server)
// ---------------------------------------------------------------------------

pub fn parse_tags_json(body: &str) -> Result<Vec<InstalledModel>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Invalid /api/tags response: {}", e))?;
    let mut models = Vec::new();
    if let Some(items) = value.get("models").and_then(|m| m.as_array()) {
        for item in items {
            let name = match item.get("name").and_then(|n| n.as_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let details = item.get("details");
            models.push(InstalledModel {
                name,
                size_bytes: item.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                modified_at: item
                    .get("modified_at")
                    .and_then(|m| m.as_str())
                    .map(String::from),
                family: details
                    .and_then(|d| d.get("family"))
                    .and_then(|f| f.as_str())
                    .map(String::from),
                parameter_size: details
                    .and_then(|d| d.get("parameter_size"))
                    .and_then(|p| p.as_str())
                    .map(String::from),
                quantization: details
                    .and_then(|d| d.get("quantization_level"))
                    .and_then(|q| q.as_str())
                    .map(String::from),
            });
        }
    }
    Ok(models)
}

pub fn parse_show_json(body: &str) -> ModelDetails {
    let mut details = ModelDetails::default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(info) = value.get("model_info").and_then(|m| m.as_object()) {
            // Context length is exposed as "<architecture>.context_length".
            for (key, val) in info {
                if key.ends_with(".context_length") {
                    details.context_length = val.as_u64();
                }
            }
        }
        details.family = value
            .get("details")
            .and_then(|d| d.get("family"))
            .and_then(|f| f.as_str())
            .map(String::from);
        details.license_excerpt = value.get("license").and_then(|l| l.as_str()).map(|l| {
            let excerpt: String = l.chars().take(200).collect();
            excerpt
        });
    }
    details
}

/// Parses one JSON line of /api/pull streaming output.
pub fn parse_pull_line(line: &str) -> Option<Result<PullProgress, String>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return None, // tolerate partial lines
    };
    if let Some(err) = value.get("error").and_then(|e| e.as_str()) {
        return Some(Err(err.to_string()));
    }
    let status = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    Some(Ok(PullProgress {
        done: status == "success",
        total_bytes: value.get("total").and_then(|t| t.as_u64()),
        completed_bytes: value.get("completed").and_then(|c| c.as_u64()),
        status,
    }))
}

pub struct ChatChunk {
    pub content: String,
    pub done: bool,
}

/// Parses one JSON line of /api/chat streaming output.
pub fn parse_chat_line(line: &str) -> Option<Result<ChatChunk, String>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return None,
    };
    if let Some(err) = value.get("error").and_then(|e| e.as_str()) {
        return Some(Err(err.to_string()));
    }
    let content = value
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let done = value.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
    Some(Ok(ChatChunk { content, done }))
}

// ---------------------------------------------------------------------------
// Network operations
// ---------------------------------------------------------------------------

pub async fn health(endpoint: &str, timeout_secs: u64) -> ProviderStatus {
    let mut status = ProviderStatus {
        provider: "ollama".to_string(),
        endpoint: endpoint.to_string(),
        reachable: false,
        version: None,
        error: None,
    };
    let client = match client(timeout_secs) {
        Ok(c) => c,
        Err(e) => {
            status.error = Some(e);
            return status;
        }
    };
    match client
        .get(format!("{}/api/version", endpoint.trim_end_matches('/')))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            status.reachable = true;
            status.version = resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("version").and_then(|s| s.as_str()).map(String::from));
        }
        Ok(resp) => {
            status.error = Some(format!("Ollama answered with HTTP {}", resp.status()));
        }
        Err(e) => {
            status.error = Some(if e.is_timeout() {
                "Timed out waiting for Ollama".to_string()
            } else {
                "Ollama is not reachable (is it installed and running?)".to_string()
            });
        }
    }
    status
}

pub async fn list_models(endpoint: &str, timeout_secs: u64) -> Result<Vec<InstalledModel>, String> {
    let client = client(timeout_secs)?;
    let body = client
        .get(format!("{}/api/tags", endpoint.trim_end_matches('/')))
        .send()
        .await
        .map_err(|e| describe_request_error(&e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read Ollama response: {}", e))?;
    parse_tags_json(&body)
}

pub async fn show_model(endpoint: &str, name: &str, timeout_secs: u64) -> ModelDetails {
    let Ok(client) = client(timeout_secs) else {
        return ModelDetails::default();
    };
    let resp = client
        .post(format!("{}/api/show", endpoint.trim_end_matches('/')))
        .json(&serde_json::json!({ "model": name }))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => match r.text().await {
            Ok(body) => parse_show_json(&body),
            Err(_) => ModelDetails::default(),
        },
        _ => ModelDetails::default(),
    }
}

pub async fn delete_model(endpoint: &str, name: &str, timeout_secs: u64) -> Result<(), String> {
    let client = client(timeout_secs)?;
    let resp = client
        .delete(format!("{}/api/delete", endpoint.trim_end_matches('/')))
        .json(&serde_json::json!({ "model": name }))
        .send()
        .await
        .map_err(|e| describe_request_error(&e))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Ollama refused the delete: HTTP {}", resp.status()))
    }
}

/// Pulls a model, reporting parsed progress lines through `on_event`.
/// Aborts (dropping the connection) when `generation` moves past
/// `my_generation`.
pub async fn pull_model(
    endpoint: &str,
    tag: &str,
    mut on_event: impl FnMut(PullProgress),
    generation: &AtomicU64,
    my_generation: u64,
) -> Result<PullOutcome, String> {
    let client = streaming_client()?;
    let resp = client
        .post(format!("{}/api/pull", endpoint.trim_end_matches('/')))
        .json(&serde_json::json!({ "model": tag, "stream": true }))
        .send()
        .await
        .map_err(|e| describe_request_error(&e))?;
    if !resp.status().is_success() {
        return Err(format!("Ollama refused the pull: HTTP {}", resp.status()));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = stream.next().await {
        if generation.load(Ordering::Relaxed) != my_generation {
            return Ok(PullOutcome::Cancelled);
        }
        let chunk = chunk.map_err(|e| format!("Download interrupted: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            if let Some(parsed) = parse_pull_line(&line) {
                let progress = parsed?;
                let done = progress.done;
                on_event(progress);
                if done {
                    return Ok(PullOutcome::Completed);
                }
            }
        }
    }
    // Stream ended without an explicit success line.
    if let Some(parsed) = parse_pull_line(&buffer) {
        let progress = parsed?;
        if progress.done {
            on_event(progress);
            return Ok(PullOutcome::Completed);
        }
    }
    Err("Download ended unexpectedly without completing".to_string())
}

/// Streams a chat completion, emitting tokens through `on_event`.
pub async fn chat_stream(
    endpoint: &str,
    model: &str,
    messages: &[ChatMessage],
    options: GenerationOptions,
    mut on_event: impl FnMut(ChatStreamEvent),
    generation: &AtomicU64,
    my_generation: u64,
) -> Result<ChatOutcome, String> {
    let client = streaming_client()?;
    let resp = client
        .post(format!("{}/api/chat", endpoint.trim_end_matches('/')))
        .timeout(Duration::from_secs(options.timeout_secs))
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": options.temperature,
                "num_predict": options.max_output_tokens,
            },
        }))
        .send()
        .await
        .map_err(|e| describe_request_error(&e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp
            .text()
            .await
            .ok()
            .and_then(|body| {
                serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            })
            .unwrap_or_default();
        return Err(format!("Ollama chat failed: HTTP {} {}", status, detail));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = stream.next().await {
        if generation.load(Ordering::Relaxed) != my_generation {
            on_event(ChatStreamEvent {
                cancelled: true,
                done: true,
                ..Default::default()
            });
            return Ok(ChatOutcome::Cancelled);
        }
        let chunk = chunk.map_err(|e| format!("Generation interrupted: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            if let Some(parsed) = parse_chat_line(&line) {
                let chunk = parsed?;
                if !chunk.content.is_empty() {
                    on_event(ChatStreamEvent {
                        token: Some(chunk.content),
                        ..Default::default()
                    });
                }
                if chunk.done {
                    on_event(ChatStreamEvent {
                        done: true,
                        ..Default::default()
                    });
                    return Ok(ChatOutcome::Completed);
                }
            }
        }
    }
    Err("Generation ended unexpectedly".to_string())
}

/// Non-streamed short prompt used to confirm a model actually answers.
pub async fn test_model(
    endpoint: &str,
    model: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let client = client(timeout_secs)?;
    let resp = client
        .post(format!("{}/api/chat", endpoint.trim_end_matches('/')))
        .json(&serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": "Reply with one short sentence confirming you are ready." }],
            "stream": false,
            "options": { "num_predict": 50 },
        }))
        .send()
        .await
        .map_err(|e| describe_request_error(&e))?;
    if !resp.status().is_success() {
        return Err(format!("Model test failed: HTTP {}", resp.status()));
    }
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Model test returned invalid JSON: {}", e))?;
    value
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Model test returned an empty response".to_string())
}

fn describe_request_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "Timed out waiting for Ollama".to_string()
    } else if e.is_connect() {
        "Ollama is not reachable (is it installed and running?)".to_string()
    } else {
        format!("Request to Ollama failed: {}", e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // -- pure parser tests --------------------------------------------------

    #[test]
    fn parses_tags_response() {
        let body = r#"{"models":[
            {"name":"qwen2.5-coder:7b","size":4683087332,"modified_at":"2026-01-01T00:00:00Z",
             "details":{"family":"qwen2","parameter_size":"7.6B","quantization_level":"Q4_K_M"}},
            {"name":"llama3.2:3b","size":2019393189}
        ]}"#;
        let models = parse_tags_json(body).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "qwen2.5-coder:7b");
        assert_eq!(models[0].size_bytes, 4683087332);
        assert_eq!(models[0].family.as_deref(), Some("qwen2"));
        assert_eq!(models[0].parameter_size.as_deref(), Some("7.6B"));
        assert_eq!(models[1].name, "llama3.2:3b");
        assert!(models[1].family.is_none());
    }

    #[test]
    fn parses_empty_and_invalid_tags() {
        assert!(parse_tags_json(r#"{"models":[]}"#).unwrap().is_empty());
        assert!(parse_tags_json("{}").unwrap().is_empty());
        assert!(parse_tags_json("not json").is_err());
    }

    #[test]
    fn parses_show_context_length() {
        let body = r#"{"details":{"family":"qwen2"},
            "model_info":{"qwen2.context_length":32768,"qwen2.embedding_length":3584},
            "license":"Apache License 2.0 ..."}"#;
        let details = parse_show_json(body);
        assert_eq!(details.context_length, Some(32768));
        assert_eq!(details.family.as_deref(), Some("qwen2"));
        assert!(details.license_excerpt.unwrap().starts_with("Apache"));
    }

    #[test]
    fn parses_pull_progress_lines() {
        let line = r#"{"status":"pulling abc","digest":"sha256:abc","total":1000,"completed":250}"#;
        let p = parse_pull_line(line).unwrap().unwrap();
        assert_eq!(p.total_bytes, Some(1000));
        assert_eq!(p.completed_bytes, Some(250));
        assert!(!p.done);

        let done = parse_pull_line(r#"{"status":"success"}"#).unwrap().unwrap();
        assert!(done.done);

        let err = parse_pull_line(r#"{"error":"pull model manifest: file does not exist"}"#)
            .unwrap()
            .unwrap_err();
        assert!(err.contains("manifest"));

        assert!(parse_pull_line("").is_none());
        assert!(parse_pull_line("{partial").is_none());
    }

    #[test]
    fn parses_chat_stream_lines() {
        let token = parse_chat_line(r#"{"message":{"role":"assistant","content":"Hel"},"done":false}"#)
            .unwrap()
            .unwrap();
        assert_eq!(token.content, "Hel");
        assert!(!token.done);

        let done = parse_chat_line(r#"{"message":{"role":"assistant","content":""},"done":true}"#)
            .unwrap()
            .unwrap();
        assert!(done.done);

        let err = parse_chat_line(r#"{"error":"model not found"}"#).unwrap().unwrap_err();
        assert!(err.contains("model not found"));
    }

    // -- network tests against a local mock listener -------------------------

    /// One-shot HTTP server on an ephemeral loopback port.
    fn mock_server(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn health_reports_reachable_with_version() {
        let endpoint = mock_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"version\":\"0.5.0\"}",
        );
        let status = health(&endpoint, 5).await;
        assert!(status.reachable, "{:?}", status.error);
        assert_eq!(status.version.as_deref(), Some("0.5.0"));
    }

    #[tokio::test]
    async fn health_reports_unreachable_endpoint() {
        // Bind-then-drop guarantees an unused port.
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let status = health(&format!("http://127.0.0.1:{}", port), 2).await;
        assert!(!status.reachable);
        assert!(status.error.unwrap().contains("not reachable"));
    }

    #[tokio::test]
    async fn list_models_times_out_against_silent_server() {
        // Server accepts but never responds -> timeout error.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                std::thread::sleep(std::time::Duration::from_secs(10));
                drop(stream);
            }
        });
        let err = list_models(&format!("http://{}", addr), 1).await.unwrap_err();
        assert!(err.contains("Timed out"), "{}", err);
    }

    #[tokio::test]
    async fn pull_model_streams_progress_and_completes() {
        let body = concat!(
            "{\"status\":\"pulling manifest\"}\n",
            "{\"status\":\"pulling layer\",\"total\":100,\"completed\":50}\n",
            "{\"status\":\"success\"}\n",
        );
        let response: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_boxed_str(),
        );
        let endpoint = mock_server(response);
        let generation = AtomicU64::new(1);
        let mut events = Vec::new();
        let outcome = pull_model(&endpoint, "tiny:latest", |p| events.push(p), &generation, 1)
            .await
            .unwrap();
        assert_eq!(outcome, PullOutcome::Completed);
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].completed_bytes, Some(50));
        assert!(events[2].done);
    }

    #[tokio::test]
    async fn pull_model_cancels_when_generation_advances() {
        let body = concat!(
            "{\"status\":\"pulling layer\",\"total\":100,\"completed\":10}\n",
            "{\"status\":\"pulling layer\",\"total\":100,\"completed\":20}\n",
        );
        let response: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_boxed_str(),
        );
        let endpoint = mock_server(response);
        let generation = AtomicU64::new(2); // already past my_generation = 1
        let outcome = pull_model(&endpoint, "tiny:latest", |_| {}, &generation, 1)
            .await
            .unwrap();
        assert_eq!(outcome, PullOutcome::Cancelled);
    }

    #[tokio::test]
    async fn pull_model_surfaces_ollama_error_lines() {
        let body = "{\"error\":\"pull model manifest: file does not exist\"}\n";
        let response: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_boxed_str(),
        );
        let endpoint = mock_server(response);
        let generation = AtomicU64::new(1);
        let err = pull_model(&endpoint, "missing:latest", |_| {}, &generation, 1)
            .await
            .unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[tokio::test]
    async fn chat_stream_emits_tokens_then_done() {
        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\" world\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n",
        );
        let response: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_boxed_str(),
        );
        let endpoint = mock_server(response);
        let generation = AtomicU64::new(1);
        let mut tokens = String::new();
        let mut saw_done = false;
        let options = GenerationOptions {
            temperature: 0.2,
            max_output_tokens: 128,
            timeout_secs: 5,
        };
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let outcome = chat_stream(
            &endpoint,
            "tiny:latest",
            &messages,
            options,
            |e| {
                if let Some(t) = e.token {
                    tokens.push_str(&t);
                }
                if e.done {
                    saw_done = true;
                }
            },
            &generation,
            1,
        )
        .await
        .unwrap();
        assert_eq!(outcome, ChatOutcome::Completed);
        assert_eq!(tokens, "Hello world");
        assert!(saw_done);
    }

    #[tokio::test]
    async fn chat_stream_cancellation_reports_cancelled_event() {
        let body = "{\"message\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"done\":false}\n";
        let response: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .into_boxed_str(),
        );
        let endpoint = mock_server(response);
        let generation = AtomicU64::new(5); // cancelled before first chunk processed
        let mut cancelled_event = false;
        let options = GenerationOptions {
            temperature: 0.2,
            max_output_tokens: 128,
            timeout_secs: 5,
        };
        let outcome = chat_stream(
            &endpoint,
            "tiny:latest",
            &[],
            options,
            |e| {
                if e.cancelled {
                    cancelled_event = true;
                }
            },
            &generation,
            1,
        )
        .await
        .unwrap();
        assert_eq!(outcome, ChatOutcome::Cancelled);
        assert!(cancelled_event);
    }
}
