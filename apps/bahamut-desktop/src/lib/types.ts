export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileNode[];
}

export interface FileTreeResponse {
  root: string;
  nodes: FileNode[];
  truncated: boolean;
}

export interface ReadFileResponse {
  path: string;
  content: string;
  hash: string;
}

export interface SaveFileResponse {
  path: string;
  new_hash: string;
  snapshot_id: number;
}

export interface RollbackResponse {
  path: string;
  restored_hash: string;
  undo_snapshot_id: number | null;
}

export interface SnapshotMeta {
  id: number;
  created_at: string;
  content_hash: string;
  size_bytes: number;
}

export interface AuditLogEntry {
  id: number;
  seq: number;
  timestamp: string;
  action_type: string;
  details: string | null;
  status: string;
  error: string | null;
  entry_hash: string;
}

export interface ChainVerification {
  valid: boolean;
  entries_checked: number;
  first_broken_seq: number | null;
  detail: string | null;
}
