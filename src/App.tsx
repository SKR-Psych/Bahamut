import { useState } from "react";
import { SetupWizard } from "./components/SetupWizard";
import { ProjectSelector } from "./components/ProjectSelector";
import "./App.css";

function App() {
  const [activeModel, setActiveModel] = useState<string | null>(null);

  const handleSetupComplete = (modelName: string) => {
    setActiveModel(modelName);
  };

  return (
    <main className="container">
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
  );
}

export default App;
