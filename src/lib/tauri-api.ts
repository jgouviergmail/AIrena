import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ArenaEvent,
  DiscussionConfig,
  DiscussionDetail,
  DiscussionSummary,
  ModelInfo,
  PredefinedProfile,
  SaveDiscussionRequest,
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

export async function preloadOllamaModel(model: string): Promise<void> {
  return await invoke("preload_ollama_model", { model });
}

// -- Settings commands --

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return await invoke("save_settings", { settings });
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
