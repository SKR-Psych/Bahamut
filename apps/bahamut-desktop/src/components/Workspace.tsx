import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  getAuditLogs,
  getSnapshotContent,
  listFileSnapshots,
  listProjectFiles,
  readProjectFile,
  rollbackFileSnapshot,
  saveProjectFile,
  setProjectRoot,
  verifyAuditChain,
} from "../lib/api";
import type {
  AppSettings,
  AuditLogEntry,
  ChainVerification,
  FileTreeResponse,
  SnapshotMeta,
} from "../lib/types";
import {
  closeAllTabs,
  closeTab,
  cycleTab,
  emptyTabs,
  focusTab,
  openTab,
  setTabDirty,
  tabsUnderPath,
  type TabsState,
} from "../state/tabs";
import { logoUrl } from "./BrandHeader";
import { TabBar } from "./TabBar";
import { EditorHost, type EditorHostHandle } from "./EditorHost";
import { FileExplorer } from "./FileExplorer";
import { SearchPanel } from "./SearchPanel";
import { SettingsPanel } from "./SettingsPanel";
import { SnapshotsPanel } from "./SnapshotsPanel";
import { AuditPanel } from "./AuditPanel";
import { ConfirmDialog } from "./ConfirmDialog";
import { DiffModal } from "./DiffModal";

interface FileBuffer {
  path: string;
  name: string;
  /** Content as last read from / written to disk. */
  content: string;
  /** Hash handed out by the backend for stale-write detection. */
  hash: string;
  /** Bumped to force the editor buffer to reset (reload / restore). */
  version: number;
}

type Activity = "files" | "search" | "settings";
type BottomTab = "snapshots" | "audit";

type Confirmation =
  | { kind: "close-tab"; path: string; name: string }
  | { kind: "close-all"; dirtyCount: number }
  | { kind: "restore"; snapshot: SnapshotMeta };

interface WorkspaceProps {
  settings: AppSettings;
  onSettingsChanged: (settings: AppSettings) => void;
}

const baseName = (path: string) => {
  const idx = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return idx >= 0 ? path.slice(idx + 1) : path;
};

export function Workspace({ settings, onSettingsChanged }: WorkspaceProps) {
  const [root, setRoot] = useState<string | null>(null);
  const [tree, setTree] = useState<FileTreeResponse | null>(null);
  const [tabs, setTabs] = useState<TabsState>(emptyTabs);
  const [buffers, setBuffers] = useState<Record<string, FileBuffer>>({});
  const [activity, setActivity] = useState<Activity>("files");
  const [selectedTreePath, setSelectedTreePath] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [conflictPath, setConflictPath] = useState<string | null>(null);
  const [conflictMessage, setConflictMessage] = useState<string | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotMeta[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLogEntry[]>([]);
  const [chain, setChain] = useState<ChainVerification | null>(null);
  const [bottomTab, setBottomTab] = useState<BottomTab>("snapshots");
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [diff, setDiff] = useState<{
    fileName: string;
    snapshotLabel: string;
    snapshotContent: string;
    currentContent: string;
  } | null>(null);
  const [pendingReveal, setPendingReveal] = useState<{ path: string; line: number } | null>(null);

  const editorRef = useRef<EditorHostHandle>(null);

  const activeTab = tabs.tabs.find((t) => t.path === tabs.activePath) ?? null;
  const activeBuffer = activeTab ? (buffers[activeTab.path] ?? null) : null;

  const refreshAudit = useCallback(async () => {
    try {
      setAuditLogs(await getAuditLogs());
      setChain(await verifyAuditChain());
    } catch (e) {
      console.warn("Could not load audit data:", e);
    }
  }, []);

  const refreshSnapshots = useCallback(async (path: string | null) => {
    if (!path) {
      setSnapshots([]);
      return;
    }
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

  useEffect(() => {
    void refreshSnapshots(tabs.activePath);
  }, [tabs.activePath, refreshSnapshots]);

  useEffect(() => {
    if (pendingReveal && tabs.activePath === pendingReveal.path) {
      editorRef.current?.revealLine(pendingReveal.path, pendingReveal.line);
      setPendingReveal(null);
    }
  }, [pendingReveal, tabs.activePath, buffers]);

  const handleSelectFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Open project folder" });
    if (typeof selected !== "string") {
      return;
    }
    try {
      const canonical = await setProjectRoot(selected);
      setRoot(canonical);
      setTabs(emptyTabs);
      setBuffers({});
      setSelectedTreePath(null);
      setConflictPath(null);
      setStatusMessage(null);
      await refreshTree();
      await refreshAudit();
    } catch (e) {
      setStatusMessage(`Could not open folder: ${e}`);
    }
  };

  const openFile = async (file: { path: string; name: string }, revealLine?: number) => {
    if (buffers[file.path]) {
      setTabs((s) => focusTab(openTab(s, file), file.path));
      if (revealLine) {
        setPendingReveal({ path: file.path, line: revealLine });
      }
      return;
    }
    try {
      const resp = await readProjectFile(file.path);
      setBuffers((b) => ({
        ...b,
        [resp.path]: {
          path: resp.path,
          name: file.name,
          content: resp.content,
          hash: resp.hash,
          version: 1,
        },
      }));
      setTabs((s) => openTab(s, { path: resp.path, name: file.name }));
      setStatusMessage(null);
      if (revealLine) {
        setPendingReveal({ path: resp.path, line: revealLine });
      }
    } catch (e) {
      setStatusMessage(`Could not open ${file.name}: ${e}`);
      await refreshAudit();
    }
  };

  const handleDirtyChange = useCallback((path: string, dirty: boolean) => {
    setTabs((s) => setTabDirty(s, path, dirty));
  }, []);

  const handleRequestSave = useCallback(
    async (path: string, text: string) => {
      const buffer = buffers[path];
      if (!buffer) {
        return;
      }
      try {
        const resp = await saveProjectFile(path, text, buffer.hash);
        setBuffers((b) => ({
          ...b,
          [path]: { ...b[path], content: text, hash: resp.new_hash },
        }));
        editorRef.current?.markSaved(path, text);
        if (conflictPath === path) {
          setConflictPath(null);
        }
        setStatusMessage(`Saved ${buffer.name}`);
        await refreshSnapshots(tabs.activePath === path ? path : tabs.activePath);
        await refreshAudit();
      } catch (e) {
        const message = String(e);
        if (message.includes("Conflict") || message.includes("no longer readable")) {
          setConflictPath(path);
          setConflictMessage(message);
        } else {
          setStatusMessage(`Save failed: ${message}`);
        }
        await refreshAudit();
      }
    },
    [buffers, conflictPath, refreshAudit, refreshSnapshots, tabs.activePath],
  );

  const reloadFromDisk = async (path: string) => {
    try {
      const resp = await readProjectFile(path);
      setBuffers((b) => ({
        ...b,
        [path]: { ...b[path], content: resp.content, hash: resp.hash, version: b[path].version + 1 },
      }));
      setConflictPath(null);
      setStatusMessage("Reloaded file from disk");
    } catch (e) {
      setStatusMessage(`Reload failed: ${e}`);
    }
  };

  const reallyCloseTab = (path: string) => {
    setTabs((s) => closeTab(s, path));
    setBuffers((b) => {
      const next = { ...b };
      delete next[path];
      return next;
    });
    if (conflictPath === path) {
      setConflictPath(null);
    }
  };

  const handleCloseRequest = (path: string) => {
    const tab = tabs.tabs.find((t) => t.path === path);
    if (!tab) {
      return;
    }
    if (tab.dirty && settings.ui_prefs.confirm_tab_close) {
      setConfirmation({ kind: "close-tab", path, name: tab.name });
    } else {
      reallyCloseTab(path);
    }
  };

  const handleCloseAllRequest = () => {
    const dirtyCount = tabs.tabs.filter((t) => t.dirty).length;
    if (dirtyCount > 0 && settings.ui_prefs.confirm_tab_close) {
      setConfirmation({ kind: "close-all", dirtyCount });
    } else {
      setTabs(closeAllTabs(tabs));
      setBuffers({});
      setConflictPath(null);
    }
  };

  const handlePathRemoved = (path: string) => {
    for (const tab of tabsUnderPath(tabs, path)) {
      reallyCloseTab(tab.path);
    }
  };

  const handlePathRenamed = (from: string, to: string) => {
    const affected = tabsUnderPath(tabs, from);
    for (const tab of affected) {
      reallyCloseTab(tab.path);
    }
    // Reopen a directly-renamed file at its new path (it was clean — the
    // explorer blocks renames under dirty tabs).
    if (affected.some((t) => t.path === from)) {
      void openFile({ path: to, name: baseName(to) });
    }
  };

  const handleRestoreRequest = (snapshot: SnapshotMeta) => {
    if (activeTab?.dirty) {
      setStatusMessage("Save or reload the file before restoring a snapshot");
      return;
    }
    setConfirmation({ kind: "restore", snapshot });
  };

  const performRestore = async (snapshot: SnapshotMeta) => {
    const path = tabs.activePath;
    if (!path) {
      return;
    }
    try {
      await rollbackFileSnapshot(snapshot.id);
      const resp = await readProjectFile(path);
      setBuffers((b) => ({
        ...b,
        [path]: { ...b[path], content: resp.content, hash: resp.hash, version: b[path].version + 1 },
      }));
      setStatusMessage("Snapshot restored — the previous content was snapshotted for undo");
      await refreshSnapshots(path);
      await refreshAudit();
    } catch (e) {
      setStatusMessage(`Restore failed: ${e}`);
      await refreshAudit();
    }
  };

  const handleDiffRequest = async (snapshot: SnapshotMeta) => {
    const path = tabs.activePath;
    if (!path || !activeBuffer) {
      return;
    }
    try {
      const snap = await getSnapshotContent(snapshot.id);
      const currentText = editorRef.current?.getText(path) ?? activeBuffer.content;
      setDiff({
        fileName: activeBuffer.name,
        snapshotLabel: `Snapshot ${snap.created_at} (${snap.operation})`,
        snapshotContent: snap.content,
        currentContent: currentText,
      });
    } catch (e) {
      setStatusMessage(`Could not load snapshot: ${e}`);
    }
  };

  const editorFiles = tabs.tabs
    .map((t) => buffers[t.path])
    .filter((b): b is FileBuffer => Boolean(b))
    .map((b) => ({ path: b.path, name: b.name, initialContent: b.content, version: b.version }));

  if (!root) {
    return (
      <div className="section-card workspace-launcher">
        <img src={logoUrl} alt="" aria-hidden="true" className="launcher-logo" width={96} height={96} />
        <h2>Open a project</h2>
        <p className="wizard-text">
          Choose a local folder to work in. All file access is locked to this folder — every
          read, write, rename, and delete is validated in the Rust backend and recorded in the
          tamper-evident audit log.
        </p>
        <button type="button" className="primary-btn" onClick={() => void handleSelectFolder()}>
          Select Project Folder
        </button>
        {statusMessage && <p className="status-error">{statusMessage}</p>}
      </div>
    );
  }

  return (
    <div
      className="workspace"
      onKeyDown={(e) => {
        if (e.ctrlKey && (e.key === "PageDown" || e.key === "PageUp")) {
          e.preventDefault();
          setTabs((s) => cycleTab(s, e.key === "PageDown" ? 1 : -1));
        }
      }}
    >
      <div className="workspace-toolbar">
        <span className="workspace-root" title={root}>
          ⛨ {root}
        </span>
        <div className="workspace-actions">
          {activeTab && (
            <button
              type="button"
              className="primary-btn btn-compact"
              disabled={!activeTab.dirty}
              onClick={() => window.dispatchEvent(new CustomEvent("bahamut:request-save"))}
            >
              {activeTab.dirty ? "Save (Ctrl+S)" : "Saved"}
            </button>
          )}
          <button
            type="button"
            className="secondary-btn btn-compact"
            onClick={() => void handleSelectFolder()}
          >
            Change Folder
          </button>
        </div>
      </div>

      {tree?.truncated && (
        <p className="banner-error">Large workspace: the file tree was truncated.</p>
      )}

      <div className="workspace-body">
        <nav className="activity-bar" aria-label="Sidebar panels">
          {(
            [
              ["files", "Files"],
              ["search", "Search"],
              ["settings", "Settings"],
            ] as [Activity, string][]
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              className={`activity-btn${activity === key ? " activity-active" : ""}`}
              aria-pressed={activity === key}
              onClick={() => setActivity(key)}
            >
              {label}
            </button>
          ))}
        </nav>

        <aside className="workspace-sidebar">
          {activity === "files" && (
            <FileExplorer
              tree={tree}
              openFilePaths={tabs.tabs.map((t) => t.path)}
              dirtyFilePaths={tabs.tabs.filter((t) => t.dirty).map((t) => t.path)}
              selectedPath={selectedTreePath}
              onSelect={(node) => setSelectedTreePath(node ? node.path : null)}
              onOpenFile={(node) => void openFile(node)}
              onTreeChanged={refreshTree}
              onPathRemoved={handlePathRemoved}
              onPathRenamed={handlePathRenamed}
              onStatus={(m) => {
                setStatusMessage(m);
                void refreshAudit();
              }}
            />
          )}
          {activity === "search" && (
            <SearchPanel
              onOpenResult={(path, name, line) => void openFile({ path, name }, line)}
            />
          )}
          {activity === "settings" && (
            <SettingsPanel settings={settings} onSettingsChanged={onSettingsChanged} />
          )}
        </aside>

        <main className="workspace-main">
          <TabBar
            tabs={tabs.tabs}
            activePath={tabs.activePath}
            onActivate={(path) => setTabs((s) => focusTab(s, path))}
            onCloseRequest={handleCloseRequest}
            onCloseAllRequest={handleCloseAllRequest}
          />
          {conflictPath && conflictPath === tabs.activePath && (
            <div className="banner-error conflict-banner">
              <span>{conflictMessage}</span>
              <button
                type="button"
                className="secondary-btn btn-compact"
                onClick={() => void reloadFromDisk(conflictPath)}
              >
                Reload from disk (discards editor changes)
              </button>
            </div>
          )}
          {tabs.tabs.length > 0 ? (
            <EditorHost
              ref={editorRef}
              files={editorFiles}
              activePath={tabs.activePath}
              onDirtyChange={handleDirtyChange}
              onRequestSave={(path, text) => void handleRequestSave(path, text)}
            />
          ) : (
            <div className="editor-placeholder">
              <img src={logoUrl} alt="" aria-hidden="true" className="placeholder-logo" width={64} height={64} />
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
            Snapshots{activeTab ? ` — ${activeTab.name}` : ""}
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
          activeTab && activeBuffer ? (
            <SnapshotsPanel
              fileName={activeTab.name}
              currentHash={activeBuffer.hash}
              snapshots={snapshots}
              onRestoreRequest={handleRestoreRequest}
              onDiffRequest={(s) => void handleDiffRequest(s)}
            />
          ) : (
            <p className="small-text">Open a file to see its snapshots.</p>
          )
        ) : (
          <AuditPanel logs={auditLogs} chain={chain} />
        )}
      </div>

      {confirmation?.kind === "close-tab" && (
        <ConfirmDialog
          title="Discard unsaved changes?"
          message={`"${confirmation.name}" has unsaved changes that will be lost if you close it.`}
          confirmLabel="Close and discard"
          danger
          onConfirm={() => {
            reallyCloseTab(confirmation.path);
            setConfirmation(null);
          }}
          onCancel={() => setConfirmation(null)}
        />
      )}
      {confirmation?.kind === "close-all" && (
        <ConfirmDialog
          title="Close all tabs?"
          message={`${confirmation.dirtyCount} tab${
            confirmation.dirtyCount === 1 ? " has" : "s have"
          } unsaved changes that will be lost.`}
          confirmLabel="Close all and discard"
          danger
          onConfirm={() => {
            setTabs(closeAllTabs(tabs));
            setBuffers({});
            setConflictPath(null);
            setConfirmation(null);
          }}
          onCancel={() => setConfirmation(null)}
        />
      )}
      {confirmation?.kind === "restore" && (
        <ConfirmDialog
          title="Restore snapshot?"
          message={`The file will be restored to the snapshot from ${confirmation.snapshot.created_at}. The current content is snapshotted first, so this restore can be undone.`}
          confirmLabel="Restore"
          onConfirm={() => {
            void performRestore(confirmation.snapshot);
            setConfirmation(null);
          }}
          onCancel={() => setConfirmation(null)}
        />
      )}

      {diff && (
        <DiffModal
          fileName={diff.fileName}
          snapshotLabel={diff.snapshotLabel}
          snapshotContent={diff.snapshotContent}
          currentContent={diff.currentContent}
          onClose={() => setDiff(null)}
        />
      )}
    </div>
  );
}
