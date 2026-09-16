import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";
import { applyGlobalContext } from "@/lib/discussion-config";

vi.mock("@/lib/tauri-api", () => ({
  clearRagStore: () => Promise.resolve(),
  saveDiscussionHistory: () => Promise.resolve(),
}));

const { useSetupStore } = await import("./useSetupStore");

const glad = (id: string) => ({
  id,
  name: id,
  interventionNumber: 0,
  systemPrompt: `<persona>${id}</persona>`,
  llmParams: { ...DEFAULT_LLM_PARAMS },
});

beforeEach(() => {
  useSetupStore.getState().reset();
});

describe("useSetupStore.buildConfig", () => {
  it("carries the document update granularity and renumbers speakers", () => {
    const s = useSetupStore.getState();
    s.addGladiateur(glad("b"));
    s.addGladiateur(glad("a"));
    s.reorderGladiateurs(1, 0);
    s.setDocumentFormat("md");
    s.setDocumentUpdateGranularity("intervention");
    const cfg = useSetupStore.getState().buildConfig("Léo");
    expect(cfg.documentUpdateGranularity).toBe("intervention");
    expect(cfg.gladiateurs.map((g) => [g.id, g.interventionNumber])).toEqual([["a", 1], ["b", 2]]);
    expect(cfg.userName).toBe("Léo");
  });

  it("defaults to per-turn document updates and resets it", () => {
    expect(useSetupStore.getState().buildConfig("x").documentUpdateGranularity).toBe("turn");
    useSetupStore.getState().setDocumentUpdateGranularity("intervention");
    useSetupStore.getState().reset();
    expect(useSetupStore.getState().documentUpdateGranularity).toBe("turn");
  });

  it("clamps search pools to turns × gladiateurs", () => {
    const s = useSetupStore.getState();
    s.addGladiateur(glad("a"));
    s.addGladiateur(glad("b"));
    s.setWebSearchPool(50);
    s.setWikiSearchPool(50);
    expect(useSetupStore.getState().webSearchPool).toBe(50);
    s.setMaxTurns(3);
    expect(useSetupStore.getState().webSearchPool).toBe(6);
    expect(useSetupStore.getState().wikiSearchPool).toBe(6);
    s.setWebSearchPool(10);
    expect(useSetupStore.getState().webSearchPool).toBe(6);
  });
});

describe("applyGlobalContext", () => {
  it("overrides every speaker's numCtx without mutating the source config", () => {
    const s = useSetupStore.getState();
    s.addGladiateur(glad("a"));
    const cfg = useSetupStore.getState().buildConfig("x");
    const out = applyGlobalContext(cfg, 32_768);
    expect(out.arbitre.llmParams.numCtx).toBe(32_768);
    expect(out.gladiateurs[0].llmParams.numCtx).toBe(32_768);
    expect(cfg.gladiateurs[0].llmParams.numCtx).toBe(DEFAULT_LLM_PARAMS.numCtx);
    expect(useSetupStore.getState().gladiateurs[0].llmParams.numCtx).toBe(DEFAULT_LLM_PARAMS.numCtx);
    // Other params untouched
    expect(out.gladiateurs[0].llmParams.temperature).toBe(DEFAULT_LLM_PARAMS.temperature);
  });
});
