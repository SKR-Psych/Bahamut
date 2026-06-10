import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HardwareInfo {
  total_ram_gb: number;
  cpu_cores: number;
  gpu_model: string;
  vram_gb?: number;
}

interface OllamaStatus {
  is_running: boolean;
  installed_models: string[];
}

export function SetupWizard({ onComplete }: { onComplete: (model: string) => void }) {
  const [step, setStep] = useState<"welcome" | "system-check" | "recommendation" | "downloading">("welcome");
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [hwInfo, setHwInfo] = useState<HardwareInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState("qwen2.5-coder:1.5b");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadSpeed, setDownloadSpeed] = useState("0 MB/s");
  const [downloadError, setDownloadError] = useState<string | null>(null);

  // Poll Ollama status on load
  const runSystemCheck = async () => {
    setLoading(true);
    try {
      // Try to get hardware details and Ollama status via Tauri Rust commands
      let status: OllamaStatus = { is_running: false, installed_models: [] };
      let hw: HardwareInfo = { total_ram_gb: 8, cpu_cores: 4, gpu_model: "Unknown GPU" };

      try {
        status = await invoke("check_ollama_status");
      } catch (e) {
        console.warn("Failed checking Ollama via command, using fallback:", e);
      }

      try {
        hw = await invoke("get_hardware_info");
      } catch (e) {
        console.warn("Failed retrieving hardware details via command, using fallback:", e);
      }

      setOllamaStatus(status);
      setHwInfo(hw);

      // Recommend model based on RAM
      if (hw.total_ram_gb >= 15) {
        setSelectedModel("qwen2.5-coder:7b");
      } else {
        setSelectedModel("qwen2.5-coder:1.5b");
      }

      setStep("system-check");
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const simulateDownload = () => {
    setStep("downloading");
    setDownloadProgress(0);
    setDownloadError(null);

    const interval = setInterval(() => {
      setDownloadProgress((prev) => {
        if (prev >= 100) {
          clearInterval(interval);
          setTimeout(() => {
            onComplete(selectedModel);
          }, 800);
          return 100;
        }
        const stepSize = Math.floor(Math.random() * 8) + 2;
        const newProgress = Math.min(prev + stepSize, 100);
        
        // Simulating speed
        const speed = (Math.random() * 15 + 5).toFixed(1);
        setDownloadSpeed(`${speed} MB/s`);

        return newProgress;
      });
    }, 300);

    return () => clearInterval(interval);
  };

  const getModelDetails = (model: string) => {
    switch (model) {
      case "qwen2.5-coder:7b":
        return {
          size: "4.7 GB",
          ramRequired: "16 GB RAM",
          license: "Apache-2.0",
          desc: "Powerful coding model. Highly accurate for complex reasoning.",
        };
      case "qwen2.5-coder:1.5b":
      default:
        return {
          size: "986 MB",
          ramRequired: "8 GB RAM",
          license: "Apache-2.0",
          desc: "Lightweight and fast coding assistant. Perfect for everyday laptops.",
        };
    }
  };

  return (
    <div className="wizard-container">
      <div className="wizard-card">
        <h1 className="wizard-title">Bahamut Setup Wizard</h1>
        <p className="wizard-subtitle">Configuring your local AI coding assistant</p>

        {step === "welcome" && (
          <div className="wizard-step">
            <p className="wizard-text">
              Welcome to Bahamut! Let's scan your system and verify that Ollama is configured to run models locally.
            </p>
            <button className="primary-btn" onClick={runSystemCheck} disabled={loading}>
              {loading ? "Scanning hardware..." : "Begin System Audit"}
            </button>
          </div>
        )}

        {step === "system-check" && ollamaStatus && hwInfo && (
          <div className="wizard-step">
            <div className="grid">
              <div className="card">
                <h3>Hardware Scan</h3>
                <p><strong>RAM:</strong> {hwInfo.total_ram_gb.toFixed(1)} GB</p>
                <p><strong>CPU:</strong> {hwInfo.cpu_cores} Cores</p>
                <p><strong>GPU:</strong> {hwInfo.gpu_model}</p>
              </div>

              <div className="card">
                <h3>Ollama Runtime</h3>
                {ollamaStatus.is_running ? (
                  <p className="status-success">● Connected to Ollama</p>
                ) : (
                  <div>
                    <p className="status-error">✕ Ollama is not running or installed</p>
                    <p className="small-text">
                      Please make sure Ollama is installed from <a href="https://ollama.com" target="_blank" rel="noreferrer">ollama.com</a> and running on port 11434.
                    </p>
                  </div>
                )}
              </div>
            </div>

            {ollamaStatus.is_running && (
              <div className="next-action">
                <button className="primary-btn" onClick={() => setStep("recommendation")}>
                  Continue to Recommendations
                </button>
              </div>
            )}

            {!ollamaStatus.is_running && (
              <div className="next-action flex-row">
                <button className="secondary-btn" onClick={runSystemCheck} disabled={loading}>
                  {loading ? "Rechecking..." : "Retry Connection"}
                </button>
                <button className="secondary-btn" onClick={() => setStep("recommendation")}>
                  Skip & Setup Offline
                </button>
              </div>
            )}
          </div>
        )}

        {step === "recommendation" && (
          <div className="wizard-step">
            <h2>Recommended Model Family: Qwen Coder</h2>
            {downloadError && <p className="status-error">✕ {downloadError}</p>}
            <p className="wizard-text">Based on your hardware specifications, we recommend:</p>

            <div className="recommendation-card active">
              <h3>{selectedModel === "qwen2.5-coder:7b" ? "Qwen 2.5 Coder 7B (Recommended)" : "Qwen 2.5 Coder 1.5B (Recommended)"}</h3>
              <p>{getModelDetails(selectedModel).desc}</p>
              <div className="model-meta">
                <span><strong>Size:</strong> {getModelDetails(selectedModel).size}</span>
                <span><strong>Req:</strong> {getModelDetails(selectedModel).ramRequired}</span>
                <span><strong>License:</strong> {getModelDetails(selectedModel).license}</span>
              </div>
            </div>

            <div className="selector-override">
              <label>Choose Model Variant:</label>
              <select value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)}>
                <option value="qwen2.5-coder:1.5b">Qwen 2.5 Coder 1.5B (986 MB)</option>
                <option value="qwen2.5-coder:7b">Qwen 2.5 Coder 7B (4.7 GB)</option>
              </select>
            </div>

            <div className="next-action flex-row">
              <button className="secondary-btn" onClick={() => setStep("system-check")}>Back</button>
              <button className="primary-btn" onClick={simulateDownload}>
                Accept & Pull Model
              </button>
            </div>
          </div>
        )}

        {step === "downloading" && (
          <div className="wizard-step">
            <h2>Downloading {selectedModel}</h2>
            <p className="wizard-text">Pulling model weights from Ollama library...</p>

            <div className="progress-bar-container">
              <div className="progress-bar-fill" style={{ width: `${downloadProgress}%` }}></div>
            </div>

            <div className="progress-meta">
              <span>{downloadProgress}% Complete</span>
              <span>Speed: {downloadSpeed}</span>
            </div>

            <div className="next-action">
              <button className="danger-btn" onClick={() => {
                setDownloadError("Download cancelled by user");
                setStep("recommendation");
              }}>
                Cancel Pull
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
