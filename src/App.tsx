import { useState } from "react";
import { SetupWizard } from "./components/SetupWizard";
import { ProjectSelector } from "./components/ProjectSelector";
import "./App.css";

function App() {
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [solidMode, setSolidMode] = useState<boolean>(false);

  const handleSetupComplete = (modelName: string) => {
    setActiveModel(modelName);
  };

  return (
    <div className={solidMode ? "accessibility-solid-mode" : ""}>
      <main className="container">
        <div className="accessibility-toggle-bar">
          <button 
            className="toggle-btn" 
            onClick={() => setSolidMode(!solidMode)}
          >
            {solidMode ? "Enable Glassmorphic Effects" : "Disable Glassmorphism (Solid Fallback)"}
          </button>
        </div>

        <div className="header-brand">
          <span className="brand-logo">✴</span>
          <h1 className="app-title">Bahamut</h1>
          <span className="badge-beta">MVP</span>
        </div>

        {!activeModel ? (
          <SetupWizard onComplete={handleSetupComplete} />
        ) : (
          <ProjectSelector activeModel={activeModel} />
        )}
      </main>
    </div>
  );
}

export default App;
