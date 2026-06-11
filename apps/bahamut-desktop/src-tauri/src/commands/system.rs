//! Hardware and runtime detection. VRAM is detected for real on Windows via
//! the display-adapter registry key (`HardwareInformation.qwMemorySize`,
//! a QWORD that is accurate above 4 GiB, unlike WMI's 32-bit AdapterRAM
//! fallback). When detection fails the profile says so honestly instead of
//! inventing a number — recommendation logic treats unknown VRAM as absent.

use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
pub struct HardwareProfile {
    pub os_name: String,
    pub os_version: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub total_ram_gb: f64,
    pub gpu_model: String,
    /// None when no reliable detection method succeeded.
    pub vram_gb: Option<f64>,
    pub vram_detected: bool,
    /// Human-readable notes about what could / could not be detected.
    pub detection_notes: Vec<String>,
}

#[tauri::command]
pub async fn get_hardware_profile() -> Result<HardwareProfile, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut notes = Vec::new();

    let total_ram_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let cpu_model = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let gpu_model = detect_gpu_model().unwrap_or_else(|| {
        notes.push("GPU model could not be detected".to_string());
        "Unknown graphics adapter".to_string()
    });

    let (vram_gb, vram_detected) = match detect_vram_bytes() {
        Some(bytes) => (
            Some((bytes as f64 / (1024.0 * 1024.0 * 1024.0) * 10.0).round() / 10.0),
            true,
        ),
        None => {
            notes.push(
                "GPU memory could not be detected reliably; recommendations use system RAM only"
                    .to_string(),
            );
            (None, false)
        }
    };

    Ok(HardwareProfile {
        os_name: System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
        os_version: System::os_version().unwrap_or_else(|| "unknown".to_string()),
        cpu_model,
        cpu_cores: sys.cpus().len(),
        total_ram_gb: (total_ram_gb * 10.0).round() / 10.0,
        gpu_model,
        vram_gb,
        vram_detected,
        detection_notes: notes,
    })
}

fn detect_gpu_model() -> Option<String> {
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

/// Best-effort VRAM detection. Returns the largest adapter's dedicated
/// memory, or None when no method produced a trustworthy value.
pub fn detect_vram_bytes() -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        if let Some(bytes) = vram_from_registry() {
            return Some(bytes);
        }
        if let Some(bytes) = vram_from_wmic() {
            return Some(bytes);
        }
    }
    None
}

/// Reads HardwareInformation.qwMemorySize for every display adapter under
/// the display class GUID; accurate for GPUs with > 4 GiB of memory.
#[cfg(target_os = "windows")]
fn vram_from_registry() -> Option<u64> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let class = hklm
        .open_subkey(
            r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
        )
        .ok()?;
    let mut best: Option<u64> = None;
    for name in class.enum_keys().flatten() {
        // Adapter instances are 4-digit subkeys (0000, 0001, …).
        if name.len() != 4 || !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(adapter) = class.open_subkey(&name) {
            if let Ok(qw) = adapter.get_value::<u64, _>("HardwareInformation.qwMemorySize") {
                if qw > 0 {
                    best = Some(best.map_or(qw, |b| b.max(qw)));
                }
            }
        }
    }
    best
}

/// 32-bit WMI fallback: caps at 4 GiB, so only trusted when the registry
/// method found nothing.
#[cfg(target_os = "windows")]
fn vram_from_wmic() -> Option<u64> {
    use std::process::Command;
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "AdapterRAM"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hardware_profile_reports_real_values_or_honest_unknowns() {
        let profile = get_hardware_profile().await.unwrap();
        assert!(profile.total_ram_gb > 0.0);
        assert!(profile.cpu_cores > 0);
        assert!(!profile.os_name.is_empty());
        // The old implementation mocked VRAM as Some(8.0) unconditionally;
        // now a value is only present when a detection method succeeded, and
        // failures must be explained in the notes.
        if profile.vram_gb.is_none() {
            assert!(!profile.vram_detected);
            assert!(profile
                .detection_notes
                .iter()
                .any(|n| n.contains("GPU memory")));
        } else {
            assert!(profile.vram_detected);
            assert!(profile.vram_gb.unwrap() > 0.0);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn vram_detection_does_not_panic() {
        // Value depends on the machine; the contract is "Some(>0) or None".
        if let Some(bytes) = detect_vram_bytes() {
            assert!(bytes > 0);
        }
    }
}
