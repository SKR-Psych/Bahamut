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
  operation: string;
}

export interface SnapshotContent {
  snapshot_id: number;
  path: string;
  content: string;
  content_hash: string;
  created_at: string;
  operation: string;
}

export interface FileOpResponse {
  path: string;
}

export interface RenameResponse {
  from: string;
  to: string;
}

export interface DeleteResponse {
  path: string;
  trash_path: string;
  snapshot_id: number | null;
}

export interface SearchOptions {
  query: string;
  case_sensitive: boolean;
  whole_word: boolean;
  regex: boolean;
  max_results?: number;
  timeout_ms?: number;
}

export interface SearchMatch {
  line: number;
  column: number;
  preview: string;
}

export interface FileSearchResult {
  path: string;
  name: string;
  matches: SearchMatch[];
}

export interface SearchResponse {
  files: FileSearchResult[];
  total_matches: number;
  files_scanned: number;
  truncated: boolean;
  timed_out: boolean;
  cancelled: boolean;
}

export interface UiPrefs {
  glassmorphism: boolean;
  solid_mode: boolean;
  confirm_tab_close: boolean;
  theme: string;
}

export interface AppSettings {
  max_file_size_bytes: number;
  max_search_file_size_bytes: number;
  ui_prefs: UiPrefs;
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
