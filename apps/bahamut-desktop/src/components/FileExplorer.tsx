import { useState } from "react";
import type { FileNode, FileTreeResponse } from "../lib/types";
import {
  createProjectFile,
  createProjectFolder,
  deleteProjectPath,
  renameProjectPath,
} from "../lib/api";
import { FileTree } from "./FileTree";
import { ConfirmDialog } from "./ConfirmDialog";

interface FileExplorerProps {
  tree: FileTreeResponse | null;
  openFilePaths: string[];
  /** Paths (file or ancestor folder) with unsaved tabs — ops are blocked. */
  dirtyFilePaths: string[];
  selectedPath: string | null;
  onSelect: (node: FileNode | null) => void;
  onOpenFile: (node: FileNode) => void;
  onTreeChanged: () => Promise<void>;
  /** Notifies the workspace that a path was renamed or deleted so affected
   *  tabs can be closed/refreshed. */
  onPathRemoved: (path: string) => void;
  onPathRenamed: (from: string, to: string) => void;
  onStatus: (message: string) => void;
}

type PendingOp =
  | { kind: "create-file" | "create-folder"; name: string }
  | { kind: "rename"; node: FileNode; name: string };

const sep = (path: string) => (path.includes("/") ? "/" : "\\");

function joinPath(dir: string, name: string): string {
  return dir.endsWith(sep(dir)) ? dir + name : dir + sep(dir) + name;
}

function parentDir(path: string): string {
  const s = sep(path);
  const idx = path.lastIndexOf(s);
  return idx > 0 ? path.slice(0, idx) : path;
}

function isUnderOrEqual(candidate: string, base: string): boolean {
  return candidate === base || candidate.startsWith(base + sep(base));
}

const NAME_PATTERN = /^[^\\/:*?"<>|]+$/;

export function FileExplorer({
  tree,
  openFilePaths,
  dirtyFilePaths,
  selectedPath,
  onSelect,
  onOpenFile,
  onTreeChanged,
  onPathRemoved,
  onPathRenamed,
  onStatus,
}: FileExplorerProps) {
  const [pending, setPending] = useState<PendingOp | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<FileNode | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedNode = findNode(tree?.nodes ?? [], selectedPath);
  const targetDir = selectedNode
    ? selectedNode.is_dir
      ? selectedNode.path
      : parentDir(selectedNode.path)
    : (tree?.root ?? null);

  function findNode(nodes: FileNode[], path: string | null): FileNode | null {
    if (!path) {
      return null;
    }
    for (const node of nodes) {
      if (node.path === path) {
        return node;
      }
      if (node.is_dir && node.children) {
        const found = findNode(node.children, path);
        if (found) {
          return found;
        }
      }
    }
    return null;
  }

  const validateName = (name: string): string | null => {
    const trimmed = name.trim();
    if (!trimmed) {
      return "Name cannot be empty";
    }
    if (!NAME_PATTERN.test(trimmed)) {
      return "Name contains characters that are not allowed";
    }
    if (trimmed === "." || trimmed === "..") {
      return "Invalid name";
    }
    return null;
  };

  const submitPending = async () => {
    if (!pending || !tree) {
      return;
    }
    const nameError = validateName(pending.name);
    if (nameError) {
      setError(nameError);
      return;
    }
    const name = pending.name.trim();
    setError(null);
    try {
      if (pending.kind === "create-file") {
        const resp = await createProjectFile(joinPath(targetDir ?? tree.root, name));
        onStatus(`Created ${resp.path}`);
      } else if (pending.kind === "create-folder") {
        const resp = await createProjectFolder(joinPath(targetDir ?? tree.root, name));
        onStatus(`Created folder ${resp.path}`);
      } else if (pending.kind === "rename") {
        const from = pending.node.path;
        const dirtyBlocked = dirtyFilePaths.some((p) => isUnderOrEqual(p, from));
        if (dirtyBlocked) {
          setError("Save or close unsaved tabs under this path first");
          return;
        }
        const to = joinPath(parentDir(from), name);
        const resp = await renameProjectPath(from, to);
        onPathRenamed(resp.from, resp.to);
        onStatus(`Renamed to ${resp.to}`);
      }
      setPending(null);
      await onTreeChanged();
    } catch (e) {
      setError(String(e));
    }
  };

  const submitDelete = async () => {
    if (!confirmDelete) {
      return;
    }
    const node = confirmDelete;
    setConfirmDelete(null);
    const dirtyBlocked = dirtyFilePaths.some((p) => isUnderOrEqual(p, node.path));
    if (dirtyBlocked) {
      setError("Save or close unsaved tabs under this path first");
      return;
    }
    try {
      const resp = await deleteProjectPath(node.path);
      onPathRemoved(node.path);
      onSelect(null);
      onStatus(`Moved to trash: ${resp.trash_path}`);
      await onTreeChanged();
    } catch (e) {
      setError(String(e));
    }
  };

  const hasOpenTabs = (path: string) => openFilePaths.some((p) => isUnderOrEqual(p, path));

  return (
    <div className="file-explorer">
      <div className="explorer-toolbar" role="toolbar" aria-label="File operations">
        <button
          type="button"
          className="secondary-btn btn-compact"
          onClick={() => {
            setError(null);
            setPending({ kind: "create-file", name: "" });
          }}
        >
          + File
        </button>
        <button
          type="button"
          className="secondary-btn btn-compact"
          onClick={() => {
            setError(null);
            setPending({ kind: "create-folder", name: "" });
          }}
        >
          + Folder
        </button>
        {selectedNode && (
          <>
            <button
              type="button"
              className="secondary-btn btn-compact"
              onClick={() => {
                setError(null);
                setPending({ kind: "rename", node: selectedNode, name: selectedNode.name });
              }}
            >
              Rename
            </button>
            <button
              type="button"
              className="danger-btn btn-compact"
              onClick={() => setConfirmDelete(selectedNode)}
            >
              Delete
            </button>
          </>
        )}
      </div>

      {pending && (
        <form
          className="explorer-inline-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submitPending();
          }}
        >
          <label className="small-text" htmlFor="explorer-name-input">
            {pending.kind === "create-file" && `New file in ${targetDir ?? ""}`}
            {pending.kind === "create-folder" && `New folder in ${targetDir ?? ""}`}
            {pending.kind === "rename" && `Rename ${pending.node.name}`}
          </label>
          <div className="flex-row">
            <input
              id="explorer-name-input"
              className="input-field input-compact"
              autoFocus
              value={pending.name}
              onChange={(e) => setPending({ ...pending, name: e.target.value })}
              placeholder="name"
            />
            <button type="submit" className="primary-btn btn-compact">
              OK
            </button>
            <button
              type="button"
              className="secondary-btn btn-compact"
              onClick={() => {
                setPending(null);
                setError(null);
              }}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {error && <p className="status-error explorer-error">{error}</p>}

      <FileTree
        nodes={tree?.nodes ?? []}
        selectedPath={selectedPath}
        onSelect={(node) => onSelect(node)}
        onOpenFile={onOpenFile}
      />

      {confirmDelete && (
        <ConfirmDialog
          title={`Delete ${confirmDelete.is_dir ? "folder" : "file"}?`}
          message={`"${confirmDelete.name}" will be moved to Bahamut's trash folder (recoverable)${
            confirmDelete.is_dir ? " together with its contents" : ""
          }${hasOpenTabs(confirmDelete.path) ? ". Its open tabs will be closed." : ""}.`}
          confirmLabel="Move to trash"
          danger
          onConfirm={() => void submitDelete()}
          onCancel={() => setConfirmDelete(null)}
        />
      )}
    </div>
  );
}
