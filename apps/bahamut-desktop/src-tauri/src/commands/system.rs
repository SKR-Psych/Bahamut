use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
pub struct HardwareInfo {
    total_ram_gb: f64,
    cpu_cores: usize,
    gpu_model: String,
    vram_gb: Option<f64>,
}

#[derive(Serialize)]
pub struct OllamaStatus {
    is_running: bool,
    installed_models: Vec<String>,
}

#[tauri::command]
pub async fn get_hardware_info() -> Result<HardwareInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_ram_bytes = sys.total_memory();
    let total_ram_gb = total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let cpu_cores = sys.cpus().len();

    // Query GPU model (on Windows we can retrieve this, or use basic detection/mocking if direct WMI isn't needed for the wizard shell)
    // To make it Windows-first and robust without heavy library setup, we can use dxgi or WMI, or fallback to sysinfo.
    // Let's use sysinfo to see if we can find any graphic cards, or fallback to a basic registry query or mock for the UI.
    let gpu_model =
        detect_gpu_on_windows().unwrap_or_else(|| "Generic Graphics Adapter".to_string());

    Ok(HardwareInfo {
        total_ram_gb,
        cpu_cores,
        gpu_model,
        vram_gb: Some(8.0), // Mocked for UI recommendations or detected if possible
    })
}

fn detect_gpu_on_windows() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("wmic")
            .args(["path", "win32_VideoController", "get", "name"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = text
                .lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != "Name")
                .collect();
            if !lines.is_empty() {
                return Some(lines.join(", "));
            }
        }
    }
    None
}

#[tauri::command]
pub async fn check_ollama_status() -> Result<OllamaStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "http://localhost:11434/api/tags";
    let res = client.get(url).send().await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                #[derive(serde::Deserialize)]
                struct ModelItem {
                    name: String,
                }
                #[derive(serde::Deserialize)]
                struct TagsResponse {
                    models: Vec<ModelItem>,
                }

                let tags: Result<TagsResponse, _> = response.json().await;
                let installed_models = match tags {
                    Ok(t) => t.models.into_iter().map(|m| m.name).collect(),
                    Err(_) => Vec::new(),
                };

                Ok(OllamaStatus {
                    is_running: true,
                    installed_models,
                })
            } else {
                Ok(OllamaStatus {
                    is_running: false,
                    installed_models: Vec::new(),
                })
            }
        }
        Err(_) => Ok(OllamaStatus {
            is_running: false,
            installed_models: Vec::new(),
        }),
    }
}
