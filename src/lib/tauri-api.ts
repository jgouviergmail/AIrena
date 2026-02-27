import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ArenaEvent,
  BudgetParams,
  DiscussionConfig,
  DiscussionDetail,
  DiscussionSummary,
  LicenseStatus,
  ModelBudgetInfo,
  ModelInfo,
  PredefinedProfile,
  RagDocumentInfo,
  SaveDiscussionRequest,
  SectionPriority,
  TokenBudgetPreview,
} from "./types";

// -- Discussion commands --

export async function startDiscussion(
  config: DiscussionConfig,
  onEvent: (event: ArenaEvent) => void,
): Promise<string> {
  const channel = new Channel<ArenaEvent>();
  channel.onmessage = onEvent;
  return await invoke<string>("start_discussion", {
    config,
    onEvent: channel,
  });
}

export async function pauseDiscussion(): Promise<void> {
  return await invoke("pause_discussion");
}

export async function resumeDiscussion(): Promise<void> {
  return await invoke("resume_discussion");
}

export async function stopDiscussion(): Promise<void> {
  return await invoke("stop_discussion");
}

export async function forceStopDiscussion(): Promise<void> {
  return await invoke("force_stop_discussion");
}

export async function userWantsToIntervene(): Promise<void> {
  return await invoke("user_wants_to_intervene");
}

export async function submitUserMessage(content: string): Promise<void> {
  return await invoke("submit_user_message", { content });
}

export async function skipUserTurn(): Promise<void> {
  return await invoke("skip_user_turn");
}

export async function adjustEmotion(
  speakerId: string,
  axis: string,
  value: number,
): Promise<void> {
  return await invoke("adjust_emotion", { speakerId, axis, value });
}

// -- Ollama commands --

export async function checkOllamaConnection(): Promise<boolean> {
  return await invoke<boolean>("check_ollama_connection");
}

export async function listOllamaModels(): Promise<ModelInfo[]> {
  return await invoke<ModelInfo[]>("list_ollama_models");
}

export async function preloadOllamaModel(model: string, numCtx?: number): Promise<void> {
  return await invoke("preload_ollama_model", { model, numCtx: numCtx ?? null });
}

export async function getModelBudgetInfo(
  model: string,
): Promise<ModelBudgetInfo> {
  return await invoke<ModelBudgetInfo>("get_model_budget_info", { model });
}

export async function initializeOllama(): Promise<ModelBudgetInfo> {
  return await invoke<ModelBudgetInfo>("initialize_ollama");
}

export async function computeTokenBudget(
  params: BudgetParams,
  priorities: SectionPriority[],
): Promise<TokenBudgetPreview> {
  return await invoke<TokenBudgetPreview>("compute_token_budget", {
    params,
    priorities,
  });
}

// -- Settings commands --

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return await invoke("save_settings", { settings });
}

// -- License commands --

export async function validateLicenseKey(key: string): Promise<LicenseStatus> {
  return await invoke<LicenseStatus>("validate_license_key", { key });
}

export async function checkLicenseStatus(): Promise<LicenseStatus> {
  return await invoke<LicenseStatus>("check_license_status");
}

export async function listProfiles(): Promise<PredefinedProfile[]> {
  return await invoke<PredefinedProfile[]>("list_profiles");
}

export async function listArbitreProfiles(): Promise<PredefinedProfile[]> {
  return await invoke<PredefinedProfile[]>("list_arbitre_profiles");
}

export async function getProfile(
  id: string,
): Promise<PredefinedProfile | null> {
  return await invoke<PredefinedProfile | null>("get_profile", { id });
}

export async function saveProfile(profile: PredefinedProfile): Promise<void> {
  return await invoke("save_profile", { profile });
}

export async function deleteProfile(id: string): Promise<void> {
  return await invoke("delete_profile", { id });
}

// -- History commands --

export async function saveDiscussionHistory(
  request: SaveDiscussionRequest,
): Promise<void> {
  return await invoke("save_discussion_history", { request });
}

export async function listDiscussionHistory(): Promise<DiscussionSummary[]> {
  return await invoke<DiscussionSummary[]>("list_discussion_history");
}

export async function getDiscussionHistory(
  id: string,
): Promise<DiscussionDetail | null> {
  return await invoke<DiscussionDetail | null>("get_discussion_history", { id });
}

export async function deleteDiscussionHistory(id: string): Promise<void> {
  return await invoke("delete_discussion_history", { id });
}

export async function deleteAllDiscussionHistory(): Promise<void> {
  return await invoke("delete_all_discussion_history");
}

// -- RAG commands --

export async function importRagDocument(
  filePath: string,
): Promise<RagDocumentInfo> {
  return await invoke<RagDocumentInfo>("import_rag_document", { filePath });
}

export async function removeRagDocument(docId: string): Promise<boolean> {
  return await invoke<boolean>("remove_rag_document", { docId });
}

export async function getRagStatus(): Promise<RagDocumentInfo[]> {
  return await invoke<RagDocumentInfo[]>("get_rag_status");
}

export async function clearRagStore(): Promise<void> {
  return await invoke("clear_rag_store");
}

// -- File utilities --

export async function downloadTextFile(
  content: string,
  defaultFileName: string,
): Promise<void> {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const { writeTextFile } = await import("@tauri-apps/plugin-fs");
  const ext = defaultFileName.split(".").pop() ?? "txt";
  const path = await save({
    defaultPath: defaultFileName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
  if (path) {
    await writeTextFile(path, content);
  }
}

/**
 * Save multiple text files to a user-chosen folder.
 * Opens a single folder picker, then writes all files into that folder.
 */
export async function downloadMultipleTextFiles(
  files: { content: string; fileName: string }[],
): Promise<void> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { writeTextFile } = await import("@tauri-apps/plugin-fs");
  const { join } = await import("@tauri-apps/api/path");
  const folder = await open({ directory: true, multiple: false });
  if (typeof folder !== "string") return;
  for (const file of files) {
    const filePath = await join(folder, file.fileName);
    await writeTextFile(filePath, file.content);
  }
}
