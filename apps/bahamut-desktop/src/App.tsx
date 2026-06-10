import { useState } from "react";
import { SetupWizard } from "./components/SetupWizard";
import { Workspace } from "./components/Workspace";
import "./App.css";

function App() {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [solidMode, setSolidMode] = useState<boolean>(false);
  const [showWizard, setShowWizard] = useState<boolean>(false);

  return (
    <div className={solidMode ? "accessibility-solid-mode" : ""}>
      <main className="container container-wide">
        <div className="accessibility-toggle-bar">
          <button className="toggle-btn" onClick={() => setShowWizard(!showWizard)}>
            {showWizard
              ? "Back to Workspace"
              : activeModel
                ? `Model: ${activeModel}`
                : "AI Model Setup"}
          </button>
          <button className="toggle-btn" onClick={() => setSolidMode(!solidMode)}>
            {solidMode
              ? "Enable Glassmorphic Effects"
              : "Disable Glassmorphism (Solid Fallback)"}
          </button>
        </div>

        <div className="header-brand">
          <span className="brand-logo">✴</span>
          <h1 className="app-title">Bahamut</h1>
          <span className="badge-beta">MVP</span>
        </div>

        {showWizard ? (
          <SetupWizard
            onComplete={(modelName) => {
              setActiveModel(modelName);
              setShowWizard(false);
            }}
          />
        ) : (
          <Workspace />
        )}
      </main>
    </div>
  );
}

export default App;
