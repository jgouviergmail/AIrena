import { create } from "zustand";
import i18n from "@/i18n/config";
import type {
  DiscussionConfig,
  GladIAteurConfig,
  IArbitreConfig,
  LlmParams,
} from "@/lib/types";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";

interface SetupState {
  step: number;
  topic: string;
  discussionLanguage: string;
  arbitre: IArbitreConfig;
  gladiateurs: GladIAteurConfig[];
  maxTurns: number | null;
  userInterventionTimeoutSecs: number;

  setStep: (step: number) => void;
  setTopic: (topic: string) => void;
  setDiscussionLanguage: (lang: string) => void;
  updateArbitre: (patch: Partial<IArbitreConfig>) => void;
  updateArbitreLlm: (patch: Partial<LlmParams>) => void;
  addGladiateur: (g: GladIAteurConfig) => void;
  removeGladiateur: (id: string) => void;
  updateGladiateur: (id: string, patch: Partial<GladIAteurConfig>) => void;
  updateGladiateurLlm: (id: string, patch: Partial<LlmParams>) => void;
  setMaxTurns: (val: number | null) => void;
  setUserTimeout: (val: number) => void;
  buildConfig: (userName: string) => DiscussionConfig;
  reset: () => void;
}

const getDefaultArbitre = (): IArbitreConfig => ({
  id: "arbitre-default",
  name: i18n.t("profiles.arb-impartial.name"),
  systemPrompt: i18n.t("profiles.arb-impartial.systemPrompt"),
  turnDistribution: "sequential",
  llmParams: { ...DEFAULT_LLM_PARAMS },
});

export const useSetupStore = create<SetupState>((set, get) => ({
  step: 0,
  topic: "",
  discussionLanguage: "fr",
  arbitre: getDefaultArbitre(),
  gladiateurs: [],
  maxTurns: null,
  userInterventionTimeoutSecs: 120,

  setStep: (step) => set({ step }),
  setTopic: (topic) => set({ topic }),
  setDiscussionLanguage: (lang) => set({ discussionLanguage: lang }),

  updateArbitre: (patch) =>
    set((s) => ({ arbitre: { ...s.arbitre, ...patch } })),
  updateArbitreLlm: (patch) =>
    set((s) => ({
      arbitre: { ...s.arbitre, llmParams: { ...s.arbitre.llmParams, ...patch } },
    })),

  addGladiateur: (g) =>
    set((s) => ({ gladiateurs: [...s.gladiateurs, g] })),
  removeGladiateur: (id) =>
    set((s) => ({
      gladiateurs: s.gladiateurs.filter((g) => g.id !== id),
    })),
  updateGladiateur: (id, patch) =>
    set((s) => ({
      gladiateurs: s.gladiateurs.map((g) =>
        g.id === id ? { ...g, ...patch } : g,
      ),
    })),
  updateGladiateurLlm: (id, patch) =>
    set((s) => ({
      gladiateurs: s.gladiateurs.map((g) =>
        g.id === id ? { ...g, llmParams: { ...g.llmParams, ...patch } } : g,
      ),
    })),

  setMaxTurns: (val) => set({ maxTurns: val }),
  setUserTimeout: (val) => set({ userInterventionTimeoutSecs: val }),

  buildConfig: (userName) => {
    const s = get();
    return {
      topic: s.topic,
      discussionLanguage: s.discussionLanguage,
      arbitre: s.arbitre,
      gladiateurs: s.gladiateurs.map((g, i) => ({
        ...g,
        interventionNumber: i + 1,
      })),
      maxTurns: s.maxTurns,
      userName,
      userInterventionTimeoutSecs: s.userInterventionTimeoutSecs,
    };
  },

  reset: () =>
    set({
      step: 0,
      topic: "",
      discussionLanguage: "fr",
      arbitre: getDefaultArbitre(),
      gladiateurs: [],
      maxTurns: null,
      userInterventionTimeoutSecs: 120,
    }),
}));
