use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};

struct AppState {
    auth_token: String,
    project_root: Mutex<Option<PathBuf>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct OllamaStatusResponse {
    is_running: bool,
    installed_models: Vec<String>,
}

#[derive(Serialize)]
struct SandboxResponse {
    sandbox_active: bool,
    workspace_name: Option<String>,
}

#[tokio::main]
async fn main() {
    // 1. Fetch random auth token from environment
    let auth_token = env::var("BAHAMUT_AUTH_TOKEN").unwrap_or_else(|_| {
        println!("Error: BAHAMUT_AUTH_TOKEN env variable not set.");
        std::process::exit(1);
    });

    let state = Arc::new(AppState {
        auth_token,
        project_root: Mutex::new(None),
    });

    // 2. Setup Stdin monitor to shut down when Electron exits
    tokio::spawn(async {
        use tokio::io::AsyncReadExt;
        let mut stdin = tokio::io::stdin();
        let mut buf = [0u8; 1];
        let _ = stdin.read(&mut buf).await;
        eprintln!("Electron connection closed. Exiting sidecar.");
        std::process::exit(0);
    });

    // 3. Build Axum Router with strict limits and timeouts
    let app = Router::new()
        .route("/v1/health", get(handle_health))
        .route("/v1/ollama/status", get(handle_ollama_status))
        .route("/v1/sandbox", get(handle_sandbox))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(RequestBodyLimitLayer::new(1024 * 16)) // Max 16KB requests
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(5))) // 5 second timeout
        .with_state(state);

    // 4. Bind strictly to 127.0.0.1 on ephemeral port 0
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let bound_addr = listener.local_addr().unwrap();

    // 5. Output bound port to stdout for Electron to read
    println!("BAHAMUT_PORT={}", bound_addr.port());

    axum::serve(listener, app).await.unwrap();
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(token) = headers.get("X-Bahamut-Auth") {
        if let Ok(token_str) = token.to_str() {
            if token_str == state.auth_token {
                return Ok(next.run(request).await);
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn handle_health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: "0.1.0",
    })
}

async fn handle_ollama_status() -> impl IntoResponse {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    let (is_running, installed_models) = match client {
        Ok(c) => {
            let res = c.get("http://localhost:11434/api/tags").send().await;
            match res {
                Ok(response) => {
                    #[derive(Deserialize)]
                    struct ModelItem {
                        name: String,
                    }
                    #[derive(Deserialize)]
                    struct TagsResponse {
                        models: Vec<ModelItem>,
                    }

                    let tags: Result<TagsResponse, _> = response.json().await;
                    let models = tags
                        .map(|t| t.models.into_iter().map(|m| m.name).collect())
                        .unwrap_or_default();
                    (true, models)
                }
                Err(_) => (false, vec![]),
            }
        }
        Err(_) => (false, vec![]),
    };

    Json(OllamaStatusResponse {
        is_running,
        installed_models,
    })
}

async fn handle_sandbox(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let root_guard = state.project_root.lock().unwrap();
    let (active, name) = match &*root_guard {
        Some(path) => (
            true,
            Some(
                path.file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Workspace".to_string()),
            ),
        ),
        None => (false, None),
    };

    Json(SandboxResponse {
        sandbox_active: active,
        workspace_name: name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_schema() {
        let resp = HealthResponse {
            status: "ok",
            version: "0.1.0",
        };
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.version, "0.1.0");
    }

    #[test]
    fn test_sandbox_response_schema() {
        let resp = SandboxResponse {
            sandbox_active: true,
            workspace_name: Some("test-project".to_string()),
        };
        assert!(resp.sandbox_active);
        assert_eq!(resp.workspace_name.unwrap(), "test-project");
    }
}
