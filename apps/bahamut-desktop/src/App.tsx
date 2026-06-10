import { useEffect, useState } from "react";
import { SetupWizard } from "./components/SetupWizard";
import { Workspace } from "./components/Workspace";
import { BrandHeader } from "./components/BrandHeader";
import { getAppSettings, updateAppSettings } from "./lib/api";
import type { AppSettings } from "./lib/types";
import "./App.css";

const DEFAULT_SETTINGS: AppSettings = {
  max_file_size_bytes: 2 * 1024 * 1024,
  max_search_file_size_bytes: 1024 * 1024,
  ui_prefs: {
    glassmorphism: true,
    solid_mode: false,
    confirm_tab_close: true,
    theme: "dark",
  },
};

function App() {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [showWizard, setShowWizard] = useState<boolean>(false);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);

  // Persisted settings drive the visual modes from startup.
  useEffect(() => {
    getAppSettings()
      .then(setSettings)
      .catch((e) => console.warn("Could not load settings, using defaults:", e));
  }, []);

  const toggleSolidMode = () => {
    const next: AppSettings = {
      ...settings,
      ui_prefs: { ...settings.ui_prefs, solid_mode: !settings.ui_prefs.solid_mode },
    };
    setSettings(next);
    // Persist best-effort; the settings panel offers full control.
    updateAppSettings(next).catch((e) => console.warn("Could not persist mode:", e));
  };

  const modeClasses = [
    settings.ui_prefs.solid_mode ? "accessibility-solid-mode" : "",
    settings.ui_prefs.glassmorphism ? "" : "no-glass",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={modeClasses}>
      <main className="container container-wide">
        <BrandHeader>
          <button className="toggle-btn" onClick={() => setShowWizard(!showWizard)}>
            {showWizard
              ? "Back to Workspace"
              : activeModel
                ? `Model: ${activeModel}`
                : "AI Model Setup"}
          </button>
          <button className="toggle-btn" onClick={toggleSolidMode}>
            {settings.ui_prefs.solid_mode
              ? "Disable Accessibility Solid Mode"
              : "Enable Accessibility Solid Mode"}
          </button>
        </BrandHeader>

        {showWizard ? (
          <SetupWizard
            onComplete={(modelName) => {
              setActiveModel(modelName);
              setShowWizard(false);
            }}
          />
        ) : (
          <Workspace settings={settings} onSettingsChanged={setSettings} />
        )}
      </main>
    </div>
  );
}

export default App;
