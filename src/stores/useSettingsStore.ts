import { create } from "zustand";
import { logger } from "@/lib/logger";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import type { AppSettings, ModelBudgetInfo, ModelInfo, PredefinedProfile } from "@/lib/types";
import * as api from "@/lib/tauri-api";

interface SettingsState {
  settings: AppSettings;
  profiles: PredefinedProfile[];
  arbitreProfiles: PredefinedProfile[];
  models: ModelInfo[];
  ollamaConnected: boolean;
  loading: boolean;
  preloading: boolean;
  preloadDone: boolean;
  preloadError: string | null;
  modelBudgetInfo: ModelBudgetInfo | null;
  modelBudgetLoading: boolean;
  initializingOllama: boolean;
  ollamaInitialized: boolean;

  hydrate: () => Promise<void>;
  updateSettings: (patch: Partial<AppSettings>) => void;
  saveSettings: () => Promise<void>;
  checkOllama: () => Promise<boolean>;
  refreshModels: () => Promise<void>;
  preloadModel: (model: string) => Promise<void>;
  fetchModelBudgetInfo: (model: string, autoFillNumCtx?: boolean) => Promise<void>;
  initializeOllama: () => Promise<void>;
  refreshProfiles: () => Promise<void>;
  saveProfile: (profile: PredefinedProfile) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  refreshArbitreProfiles: () => Promise<void>;
  saveArbitreProfile: (profile: PredefinedProfile) => Promise<void>;
  deleteArbitreProfile: (id: string) => Promise<void>;
}

let preloadGeneration = 0;
let budgetInfoGeneration = 0;

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: {
    username: "",
    language: "fr",
    theme: "dark",
    ollamaUrl: "http://localhost:11434",
    ollamaModel: "",
    emotionDriven: false,
    tavilyApiKey: "",
    tavilyPeriodStart: "",
    tavilyUsageCount: 0,
    tavilyUsageHistory: "[]",
    embeddingModel: "",
    licenseKey: "",
    tokenBudgetPriorities: "",
    numCtx: 8192,
  },
  profiles: [],
  arbitreProfiles: [],
  models: [],
  ollamaConnected: false,
  loading: true,
  preloading: false,
  preloadDone: false,
  preloadError: null,
  modelBudgetInfo: null,
  modelBudgetLoading: false,
  initializingOllama: false,
  ollamaInitialized: false,

  hydrate: async () => {
    try {
      const [settings, profiles, arbitreProfiles] = await Promise.all([
        api.getSettings(),
        api.listProfiles(),
        api.listArbitreProfiles(),
      ]);
      set({ settings, profiles, arbitreProfiles, loading: false });
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
    const { settings } = get();
    await api.saveSettings(settings);
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
      const numCtx = get().settings.numCtx;
      await api.preloadOllamaModel(model, numCtx > 0 ? numCtx : undefined);
      if (gen === preloadGeneration) {
        logger.info("settings", `Model preloaded: ${model}`);
        set({ preloading: false, preloadDone: true });
        // After successful preload, refresh VRAM display (model is now loaded, VRAM changed)
        get().fetchModelBudgetInfo(model, false);
      } else {
        logger.debug("settings", `Stale preload ignored (gen=${gen}, current=${preloadGeneration})`);
      }
    } catch (e) {
      logger.error("settings", `Failed to preload model: ${model}`, e);
      if (gen === preloadGeneration) {
        set({ preloading: false, preloadError: extractErrorMessage(e) });
      }
    }
  },

  fetchModelBudgetInfo: async (model, autoFillNumCtx = false) => {
    if (!model) {
      set({ modelBudgetInfo: null, modelBudgetLoading: false });
      return;
    }
    const gen = ++budgetInfoGeneration;
    set({ modelBudgetLoading: true });
    try {
      const info = await api.getModelBudgetInfo(model);
      if (gen === budgetInfoGeneration) {
        set({ modelBudgetInfo: info, modelBudgetLoading: false });
        // Only auto-fill numCtx when explicitly requested (user changed model)
        if (autoFillNumCtx && info.recommendedNumCtx) {
          get().updateSettings({ numCtx: info.recommendedNumCtx });
        }
      }
    } catch (e) {
      logger.error("settings", `Failed to fetch model budget info: ${model}`, e);
      if (gen === budgetInfoGeneration) {
        set({ modelBudgetInfo: null, modelBudgetLoading: false });
      }
    }
  },

  initializeOllama: async () => {
    set({ initializingOllama: true });
    try {
      const info = await api.initializeOllama();
      set({
        ollamaConnected: true,
        modelBudgetInfo: info,
        preloadDone: true,
        ollamaInitialized: true,
        initializingOllama: false,
      });
      // Auto-fill numCtx with recommended value
      if (info.recommendedNumCtx) {
        get().updateSettings({ numCtx: info.recommendedNumCtx });
      }
      // Populate model list
      await get().refreshModels();
      logger.info("settings", "Ollama initialized successfully");
    } catch (e) {
      logger.error("settings", "Ollama initialization failed, falling back to checkOllama", e);
      set({ initializingOllama: false });
      // Fallback: at least check connectivity
      await get().checkOllama();
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
      toast.error("Failed to save profile", extractErrorMessage(e));
    }
  },

  deleteProfile: async (id) => {
    try {
      await api.deleteProfile(id);
      await get().refreshProfiles();
    } catch (e) {
      toast.error("Failed to delete profile", extractErrorMessage(e));
    }
  },

  refreshArbitreProfiles: async () => {
    try {
      const arbitreProfiles = await api.listArbitreProfiles();
      set({ arbitreProfiles });
    } catch (e) {
      console.error("Failed to refresh arbitre profiles:", e);
    }
  },

  saveArbitreProfile: async (profile) => {
    try {
      await api.saveProfile(profile);
      await get().refreshArbitreProfiles();
    } catch (e) {
      toast.error("Failed to save arbitre profile", extractErrorMessage(e));
    }
  },

  deleteArbitreProfile: async (id) => {
    try {
      await api.deleteProfile(id);
      await get().refreshArbitreProfiles();
    } catch (e) {
      toast.error("Failed to delete arbitre profile", extractErrorMessage(e));
    }
  },
}));
