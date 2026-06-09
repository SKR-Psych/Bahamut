import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ProjectSelector({ activeModel }: { activeModel: string }) {
  const [projectRoot, setProjectRoot] = useState<string>("");
  const [currentRoot, setCurrentRoot] = useState<string>("");
  const [checkPath, setCheckPath] = useState<string>("");
  const [checkResult, setCheckResult] = useState<{ success: boolean; message: string } | null>(null);
  const [auditLogs, setAuditLogs] = useState<any[]>([]);

  const handleOpenFolder = async () => {
    try {
      // Set project root directory via Rust backend
      const result: string = await invoke("set_project_root", { path: projectRoot });
      setCurrentRoot(result);
      loadAuditLogs();
    } catch (e: any) {
      alert(`Error opening folder: ${e}`);
    }
  };

  const handleCheckSandbox = async () => {
    try {
      await invoke("check_file_in_sandbox", { path: checkPath });
      setCheckResult({ success: true, message: `Access GRANTED. Path is within project root.` });
    } catch (e: any) {
      setCheckResult({ success: false, message: `Access DENIED: ${e}` });
    }
    loadAuditLogs();
  };

  const loadAuditLogs = async () => {
    try {
      const logs: any[] = await invoke("get_audit_logs");
      setAuditLogs(logs);
    } catch (e) {
      console.warn("Could not load audit logs:", e);
    }
  };

  useEffect(() => {
    if (currentRoot) {
      loadAuditLogs();
    }
  }, [currentRoot]);

  return (
    <div className="main-panel">
      <div className="section-card">
        <h2>Active Project Workspace</h2>
        <p className="small-text">Current Active Model: <code>{activeModel}</code></p>
        
        <div className="flex-row">
          <input
            type="text"
            placeholder="Type path (e.g. C:\Users\Sami\Documents\project)"
            value={projectRoot}
            onChange={(e) => setProjectRoot(e.target.value)}
            className="input-field"
          />
          <button className="primary-btn" onClick={handleOpenFolder}>
            Open Workspace
          </button>
        </div>

        {currentRoot && (
          <p className="success-banner">
            ✓ Sandbox Root Locked: <code>{currentRoot}</code>
          </p>
        )}
      </div>

      {currentRoot && (
        <>
          <div className="section-card">
            <h2>Sandbox Boundary Test</h2>
            <p className="small-text">Test sandbox enforcement by validating absolute paths or traversal sequences.</p>
            
            <div className="flex-row">
              <input
                type="text"
                placeholder="Check target path (e.g. C:\Windows\System32 or ..\outside)"
                value={checkPath}
                onChange={(e) => setCheckPath(e.target.value)}
                className="input-field"
              />
              <button className="secondary-btn" onClick={handleCheckSandbox}>
                Validate Path Safety
              </button>
            </div>

            {checkResult && (
              <div className={checkResult.success ? "banner-success" : "banner-error"}>
                {checkResult.message}
              </div>
            )}
          </div>

          <div className="section-card">
            <h2>Security Audit Log</h2>
            <p className="small-text">Real-time local audits from SQLite database</p>
            
            <div className="table-container">
              <table className="audit-table">
                <thead>
                  <tr>
                    <th>Timestamp</th>
                    <th>Action</th>
                    <th>Details</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {auditLogs.length === 0 ? (
                    <tr>
                      <td colSpan={4} style={{ textAlign: "center" }}>No logs recorded yet.</td>
                    </tr>
                  ) : (
                    auditLogs.map((log) => (
                      <tr key={log.id}>
                        <td>{log.timestamp}</td>
                        <td><code>{log.action_type}</code></td>
                        <td>{log.details || "-"}</td>
                        <td>
                          <span className={log.status === "success" ? "badge-success" : "badge-error"}>
                            {log.status}
                          </span>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
