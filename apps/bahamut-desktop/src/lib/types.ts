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
  ai: AiSettings;
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

export interface AiSettings {
  local_ai_enabled: boolean;
  provider: "ollama";
  active_model: string | null;
  context_limit: number;
  per_file_attachment_limit: number;
  history_persistence: boolean;
  ollama_endpoint: string;
  request_timeout_ms: number;
  temperature: number;
  max_output_tokens: number;
}
export interface HardwareProfile { total_ram_gb: number; cpu_cores: number; gpu_model: string | null; vram_gb: number | null; tier: string; detection_notes: string[]; }
export interface ModelCatalogueEntry { id: string; provider: string; display_name: string; family: string; parameters_b: number; quantization: string; context_window: number; download_size_gb: number; min_ram_gb: number; recommended_ram_gb: number; min_vram_gb: number | null; recommended_vram_gb: number | null; license: string; license_url: string; safety_notes: string; tags: string[]; }
export interface ModelRecommendation { model: ModelCatalogueEntry; fit: string; reasons: string[]; warnings: string[]; }
export interface ProviderStatus { provider: string; reachable: boolean; version: string | null; message: string; }
export interface InstalledModel { id: string; display_name: string; size_bytes: number | null; modified_at: string | null; digest: string | null; details: unknown; }
export interface ChatMessage { role: "system" | "user" | "assistant"; content: string; }
export interface ChatRequest { conversation_id?: number | null; model: string; messages: ChatMessage[]; temperature?: number; max_output_tokens?: number; }
export interface Conversation { id: number; title: string; model: string | null; created_at: string; updated_at: string; }
export interface StoredMessage { id: number; conversation_id: number; role: string; content: string; model: string | null; attachment_metadata: string | null; status: string; error: string | null; created_at: string; }
export interface ConversationDetail { conversation: Conversation; messages: StoredMessage[]; }
export interface AttachmentRequest { kind: "open_file" | "selected_file" | "selected_text" | "search_result" | "manual_text"; path?: string | null; label?: string | null; text?: string | null; start_line?: number | null; end_line?: number | null; }
export interface SecretFinding { category: string; label: string; line: number | null; column: number | null; }
export interface Attachment { kind: string; path: string | null; label: string; content: string; original_bytes: number; included_bytes: number; truncated: boolean; secret_findings: SecretFinding[]; }
export interface ContextAssembly { attachments: Attachment[]; total_bytes: number; total_limit: number; truncated: boolean; system_boundary: string; }
export interface StoredDataSummary { conversations: number; messages: number; approximate_bytes: number; persistence_enabled: boolean; }
