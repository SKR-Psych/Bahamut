import { invoke } from "@tauri-apps/api/core";
import type {
  AuditLogEntry,
  ChainVerification,
  FileTreeResponse,
  ReadFileResponse,
  RollbackResponse,
  SaveFileResponse,
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
