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

export function getHardwareProfile(): Promise<import("./types").HardwareProfile> { return invoke("get_hardware_profile"); }
export function getModelCatalogue(): Promise<import("./types").ModelCatalogueEntry[]> { return invoke("get_model_catalogue"); }
export function getModelRecommendations(profile?: import("./types").HardwareProfile): Promise<import("./types").ModelRecommendation[]> { return invoke("get_model_recommendations", { profile: profile ?? null }); }
export function getProviderStatus(): Promise<import("./types").ProviderStatus> { return invoke("get_provider_status"); }
export function reconnectProvider(): Promise<import("./types").ProviderStatus> { return invoke("reconnect_provider"); }
export function listInstalledModels(): Promise<import("./types").InstalledModel[]> { return invoke("list_installed_models"); }
export function pullModel(model: string): Promise<void> { return invoke("pull_model", { model }); }
export function cancelModelPull(): Promise<void> { return invoke("cancel_model_pull"); }
export function deleteModel(model: string): Promise<void> { return invoke("delete_model", { model }); }
export function selectActiveModel(model: string | null): Promise<AppSettings> { return invoke("select_active_model", { model }); }
export function testPrompt(model: string): Promise<string> { return invoke("test_prompt", { model }); }
export function startChat(request: import("./types").ChatRequest): Promise<string> { return invoke("start_chat", { request }); }
export function cancelChat(): Promise<void> { return invoke("cancel_chat"); }
export function assembleChatContext(attachments: import("./types").AttachmentRequest[]): Promise<import("./types").ContextAssembly> { return invoke("assemble_chat_context", { attachments }); }
export function approveSecretContext(categories: string[], attachmentCount: number): Promise<void> { return invoke("approve_secret_context", { categories, attachmentCount }); }
export function createConversation(title: string, model?: string | null): Promise<import("./types").Conversation> { return invoke("create_conversation", { title, model: model ?? null }); }
export function listConversations(): Promise<import("./types").Conversation[]> { return invoke("list_conversations"); }
export function readConversation(id: number): Promise<import("./types").ConversationDetail> { return invoke("read_conversation", { id }); }
export function renameConversation(id: number, title: string): Promise<import("./types").Conversation> { return invoke("rename_conversation", { id, title }); }
export function deleteConversation(id: number): Promise<void> { return invoke("delete_conversation", { id }); }
export function clearConversationHistory(): Promise<void> { return invoke("clear_conversation_history"); }
export function inspectStoredChatData(): Promise<import("./types").StoredDataSummary> { return invoke("inspect_stored_chat_data"); }
