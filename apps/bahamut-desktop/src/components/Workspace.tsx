import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  getAuditLogs,
  listFileSnapshots,
  listProjectFiles,
  readProjectFile,
  rollbackFileSnapshot,
  saveProjectFile,
  setProjectRoot,
  verifyAuditChain,
} from "../lib/api";
import type {
  AuditLogEntry,
  ChainVerification,
  FileNode,
  FileTreeResponse,
  SnapshotMeta,
} from "../lib/types";
import { FileTree } from "./FileTree";
import { EditorPane } from "./EditorPane";
import { SnapshotsPanel } from "./SnapshotsPanel";
import { AuditPanel } from "./AuditPanel";

interface OpenFile {
  path: string;
  name: string;
  content: string;
  hash: string;
}

type BottomTab = "snapshots" | "audit";

export function Workspace() {
  const [root, setRoot] = useState<string | null>(null);
  const [tree, setTree] = useState<FileTreeResponse | null>(null);
  const [openFile, setOpenFile] = useState<OpenFile | null>(null);
  const [contentVersion, setContentVersion] = useState(0);
  const [dirty, setDirty] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [conflictMessage, setConflictMessage] = useState<string | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotMeta[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLogEntry[]>([]);
  const [chain, setChain] = useState<ChainVerification | null>(null);
  const [bottomTab, setBottomTab] = useState<BottomTab>("snapshots");

  const refreshAudit = useCallback(async () => {
    try {
      setAuditLogs(await getAuditLogs());
      setChain(await verifyAuditChain());
    } catch (e) {
      console.warn("Could not load audit data:", e);
    }
  }, []);

  const refreshSnapshots = useCallback(async (path: string) => {
    try {
      setSnapshots(await listFileSnapshots(path));
    } catch (e) {
      console.warn("Could not load snapshots:", e);
      setSnapshots([]);
    }
  }, []);

  const refreshTree = useCallback(async () => {
    try {
      setTree(await listProjectFiles());
    } catch (e) {
      setStatusMessage(`Failed to list project files: ${e}`);
    }
  }, []);

  const handleSelectFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Open project folder" });
    if (typeof selected !== "string") {
      return; // user cancelled
    }
    try {
      const canonical = await setProjectRoot(selected);
      setRoot(canonical);
      setOpenFile(null);
      setDirty(false);
      setConflictMessage(null);
      await refreshTree();
      await refreshAudit();
    } catch (e) {
      setStatusMessage(`Could not open folder: ${e}`);
    }
  };

  const handleOpenFile = async (node: FileNode) => {
    if (dirty && openFile && !window.confirm("Discard unsaved changes?")) {
      return;
    }
    try {
      const resp = await readProjectFile(node.path);
      setOpenFile({ path: resp.path, name: node.name, content: resp.content, hash: resp.hash });
      setContentVersion((v) => v + 1);
      setConflictMessage(null);
      setStatusMessage(null);
      await refreshSnapshots(resp.path);
    } catch (e) {
      setStatusMessage(`Could not open ${node.name}: ${e}`);
      await refreshAudit();
    }
  };

  const handleSave = async (currentText: string) => {
    if (!openFile) {
      return;
    }
    try {
      const resp = await saveProjectFile(openFile.path, currentText, openFile.hash);
      setOpenFile({ ...openFile, content: currentText, hash: resp.new_hash });
      setDirty(false);
      setConflictMessage(null);
      setStatusMessage(`Saved ${openFile.name}`);
      await refreshSnapshots(openFile.path);
      await refreshAudit();
    } catch (e) {
      const message = String(e);
      if (message.includes("Conflict") || message.includes("no longer readable")) {
        setConflictMessage(message);
      } else {
        setStatusMessage(`Save failed: ${message}`);
      }
      await refreshAudit();
    }
  };

  const handleReloadFromDisk = async () => {
    if (!openFile) {
      return;
    }
    try {
      const resp = await readProjectFile(openFile.path);
      setOpenFile({ ...openFile, content: resp.content, hash: resp.hash });
      setContentVersion((v) => v + 1);
      setConflictMessage(null);
      setStatusMessage("Reloaded file from disk");
    } catch (e) {
      setStatusMessage(`Reload failed: ${e}`);
    }
  };

  const handleRestoreSnapshot = async (snapshotId: number) => {
    if (!window.confirm("Restore this snapshot? The current on-disk content is snapshotted first, so this can be undone.")) {
      return;
    }
    try {
      await rollbackFileSnapshot(snapshotId);
      setStatusMessage("Snapshot restored");
      setConflictMessage(null);
      if (openFile) {
        const resp = await readProjectFile(openFile.path);
        setOpenFile({ ...openFile, content: resp.content, hash: resp.hash });
        setContentVersion((v) => v + 1);
        await refreshSnapshots(openFile.path);
      }
      await refreshAudit();
    } catch (e) {
      setStatusMessage(`Restore failed: ${e}`);
      await refreshAudit();
    }
  };

  useEffect(() => {
    if (root) {
      void refreshAudit();
    }
  }, [root, refreshAudit]);

  if (!root) {
    return (
      <div className="section-card workspace-launcher">
        <h2>Open a project</h2>
        <p className="wizard-text">
          Choose a local folder to work in. All file access is locked to this folder — every
          read and write is validated in the Rust backend and recorded in the tamper-evident
          audit log.
        </p>
        <button type="button" className="primary-btn" onClick={handleSelectFolder}>
          Select Project Folder
        </button>
        {statusMessage && <p className="status-error">{statusMessage}</p>}
      </div>
    );
  }

  return (
    <div className="workspace">
      <div className="workspace-toolbar">
        <span className="workspace-root" title={root}>
          ⛨ {root}
        </span>
        <div className="workspace-actions">
          {openFile && (
            <button
              type="button"
              className="primary-btn btn-compact"
              disabled={!dirty}
              onClick={() => {
                // EditorPane owns the buffer and listens for this event; it
                // invokes the same save path as Ctrl+S with the live text.
                window.dispatchEvent(new CustomEvent("bahamut:request-save"));
              }}
            >
              {dirty ? "Save (Ctrl+S)" : "Saved"}
            </button>
          )}
          <button type="button" className="secondary-btn btn-compact" onClick={handleSelectFolder}>
            Change Folder
          </button>
        </div>
      </div>

      {tree?.truncated && (
        <p className="banner-error">Large workspace: the file tree was truncated.</p>
      )}

      <div className="workspace-body">
        <aside className="workspace-sidebar">
          <FileTree
            nodes={tree?.nodes ?? []}
            selectedPath={openFile?.path ?? null}
            onOpenFile={handleOpenFile}
          />
        </aside>

        <main className="workspace-main">
          {conflictMessage && (
            <div className="banner-error conflict-banner">
              <span>{conflictMessage}</span>
              <button
                type="button"
                className="secondary-btn btn-compact"
                onClick={handleReloadFromDisk}
              >
                Reload from disk (discards editor changes)
              </button>
            </div>
          )}
          {openFile ? (
            <EditorPane
              filePath={openFile.path}
              fileName={openFile.name}
              content={openFile.content}
              contentVersion={contentVersion}
              onDirtyChange={setDirty}
              onRequestSave={handleSave}
            />
          ) : (
            <div className="editor-placeholder">
              <p className="wizard-text">Select a file from the tree to open it in the editor.</p>
            </div>
          )}
        </main>
      </div>

      <div className="workspace-bottom">
        <div className="bottom-tabs">
          <button
            type="button"
            className={`tab-btn${bottomTab === "snapshots" ? " tab-active" : ""}`}
            onClick={() => setBottomTab("snapshots")}
          >
            Snapshots{openFile ? ` — ${openFile.name}` : ""}
          </button>
          <button
            type="button"
            className={`tab-btn${bottomTab === "audit" ? " tab-active" : ""}`}
            onClick={() => setBottomTab("audit")}
          >
            Audit Log
          </button>
          {statusMessage && <span className="status-inline">{statusMessage}</span>}
        </div>
        {bottomTab === "snapshots" ? (
          openFile ? (
            <SnapshotsPanel snapshots={snapshots} onRestore={handleRestoreSnapshot} />
          ) : (
            <p className="small-text">Open a file to see its snapshots.</p>
          )
        ) : (
          <AuditPanel logs={auditLogs} chain={chain} />
        )}
      </div>
    </div>
  );
}
