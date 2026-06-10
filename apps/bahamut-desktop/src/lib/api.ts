import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AuditLogEntry,
  ChainVerification,
  DeleteResponse,
  FileOpResponse,
  FileTreeResponse,
  ReadFileResponse,
  RenameResponse,
  RollbackResponse,
  SaveFileResponse,
  SearchOptions,
  SearchResponse,
  SnapshotContent,
  SnapshotMeta,
} from "./types";

/** Typed wrappers around the narrow Rust command surface. */

export function setProjectRoot(path: string): Promise<string> {
  return invoke("set_project_root", { path });
}

export function listProjectFiles(): Promise<FileTreeResponse> {
  return invoke("list_project_files");
}

export function readProjectFile(path: string): Promise<ReadFileResponse> {
  return invoke("read_project_file", { path });
}

export function saveProjectFile(
  path: string,
  content: string,
  expectedHash: string,
): Promise<SaveFileResponse> {
  return invoke("save_project_file", { path, content, expectedHash });
}

export function rollbackFileSnapshot(snapshotId: number): Promise<RollbackResponse> {
  return invoke("rollback_file_snapshot", { snapshotId });
}

export function listFileSnapshots(path: string): Promise<SnapshotMeta[]> {
  return invoke("list_file_snapshots", { path });
}

export function getAuditLogs(): Promise<AuditLogEntry[]> {
  return invoke("get_audit_logs");
}

export function verifyAuditChain(): Promise<ChainVerification> {
  return invoke("verify_audit_chain");
}

export function getSnapshotContent(snapshotId: number): Promise<SnapshotContent> {
  return invoke("get_snapshot_content", { snapshotId });
}

export function createProjectFile(path: string): Promise<FileOpResponse> {
  return invoke("create_project_file", { path });
}

export function createProjectFolder(path: string): Promise<FileOpResponse> {
  return invoke("create_project_folder", { path });
}

export function renameProjectPath(from: string, to: string): Promise<RenameResponse> {
  return invoke("rename_project_path", { from, to });
}

export function deleteProjectPath(path: string): Promise<DeleteResponse> {
  return invoke("delete_project_path", { path });
}

export function searchProject(options: SearchOptions): Promise<SearchResponse> {
  return invoke("search_project", { options });
}

export function cancelProjectSearch(): Promise<void> {
  return invoke("cancel_project_search");
}

export function getAppSettings(): Promise<AppSettings> {
  return invoke("get_app_settings");
}

export function updateAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke("update_app_settings", { settings });
}

export function resetAppSettings(): Promise<AppSettings> {
  return invoke("reset_app_settings");
}
