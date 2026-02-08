import { create } from "zustand";
import { logger } from "@/lib/logger";
import type { AppSettings, ModelInfo, PredefinedProfile } from "@/lib/types";
import * as api from "@/lib/tauri-api";

interface SettingsState {
  settings: AppSettings;
  profiles: PredefinedProfile[];
  models: ModelInfo[];
  ollamaConnected: boolean;
  loading: boolean;
  preloading: boolean;
  preloadDone: boolean;
  preloadError: string | null;

  hydrate: () => Promise<void>;
  updateSettings: (patch: Partial<AppSettings>) => void;
  saveSettings: () => Promise<void>;
  checkOllama: () => Promise<boolean>;
  refreshModels: () => Promise<void>;
  preloadModel: (model: string) => Promise<void>;
  refreshProfiles: () => Promise<void>;
  saveProfile: (profile: PredefinedProfile) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
}

let preloadGeneration = 0;

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: {
    username: "",
    language: "fr",
    theme: "dark",
    ollamaUrl: "http://localhost:11434",
    ollamaModel: "",
    emotionDriven: false,
  },
  profiles: [],
  models: [],
  ollamaConnected: false,
  loading: true,
  preloading: false,
  preloadDone: false,
  preloadError: null,

  hydrate: async () => {
    try {
      const [settings, profiles] = await Promise.all([
        api.getSettings(),
        api.listProfiles(),
      ]);
      set({ settings, profiles, loading: false });
    } catch (e) {
      console.error("Failed to hydrate settings:", e);
      set({ loading: false });
    }
  },

  updateSettings: (patch) => {
    set((s) => ({
      settings: { ...s.settings, ...patch },
      // Reset preload indicators when model changes
      ...(patch.ollamaModel !== undefined
        ? { preloadDone: false, preloadError: null }
        : {}),
    }));
  },

  saveSettings: async () => {
    try {
      const { settings } = get();
      await api.saveSettings(settings);
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
  },

  checkOllama: async () => {
    try {
      const connected = await api.checkOllamaConnection();
      set({ ollamaConnected: connected });
      if (connected) {
        await get().refreshModels();
      }
      return connected;
    } catch {
      set({ ollamaConnected: false });
      return false;
    }
  },

  refreshModels: async () => {
    try {
      const models = await api.listOllamaModels();
      set({ models });
    } catch {
      set({ models: [] });
    }
  },

  preloadModel: async (model) => {
    if (!model) return;
    const gen = ++preloadGeneration;
    logger.info("settings", `Preloading model: ${model} (gen=${gen})`);
    set({ preloading: true, preloadDone: false, preloadError: null });
    try {
      await api.preloadOllamaModel(model);
      if (gen === preloadGeneration) {
        logger.info("settings", `Model preloaded: ${model}`);
        set({ preloading: false, preloadDone: true });
      } else {
        logger.debug("settings", `Stale preload ignored (gen=${gen}, current=${preloadGeneration})`);
      }
    } catch (e) {
      logger.error("settings", `Failed to preload model: ${model}`, e);
      if (gen === preloadGeneration) {
        set({ preloading: false, preloadError: String(e) });
      }
    }
  },

  refreshProfiles: async () => {
    try {
      const profiles = await api.listProfiles();
      set({ profiles });
    } catch (e) {
      console.error("Failed to refresh profiles:", e);
    }
  },

  saveProfile: async (profile) => {
    try {
      await api.saveProfile(profile);
      await get().refreshProfiles();
    } catch (e) {
      console.error("Failed to save profile:", e);
    }
  },

  deleteProfile: async (id) => {
    try {
      await api.deleteProfile(id);
      await get().refreshProfiles();
    } catch (e) {
      console.error("Failed to delete profile:", e);
    }
  },
}));
