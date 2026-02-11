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
  webSearchPool: number;
  wikiSearchPool: number;

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
  setWebSearchPool: (val: number) => void;
  setWikiSearchPool: (val: number) => void;
  reorderGladiateurs: (fromIndex: number, toIndex: number) => void;
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
  webSearchPool: 0,
  wikiSearchPool: 0,

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

  setMaxTurns: (val) =>
    set((s) => {
      const poolMax = val !== null ? val * Math.max(s.gladiateurs.length, 1) : Infinity;
      return {
        maxTurns: val,
        // Auto-clamp pool if turns decreased below it
        webSearchPool: s.webSearchPool > poolMax ? poolMax : s.webSearchPool,
        wikiSearchPool: s.wikiSearchPool > poolMax ? poolMax : s.wikiSearchPool,
      };
    }),
  setUserTimeout: (val) => set({ userInterventionTimeoutSecs: val }),
  setWebSearchPool: (val) =>
    set((s) => {
      const poolMax = (s.maxTurns ?? Infinity) * Math.max(s.gladiateurs.length, 1);
      return { webSearchPool: Math.min(val, poolMax) };
    }),
  setWikiSearchPool: (val) =>
    set((s) => {
      const poolMax = (s.maxTurns ?? Infinity) * Math.max(s.gladiateurs.length, 1);
      return { wikiSearchPool: Math.min(val, poolMax) };
    }),

  reorderGladiateurs: (fromIndex, toIndex) =>
    set((s) => {
      const list = [...s.gladiateurs];
      const [moved] = list.splice(fromIndex, 1);
      list.splice(toIndex, 0, moved);
      return { gladiateurs: list };
    }),

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
      webSearchPool: s.webSearchPool,
      wikiSearchPool: s.wikiSearchPool,
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
      webSearchPool: 0,
      wikiSearchPool: 0,
    }),
}));
