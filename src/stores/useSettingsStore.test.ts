import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppSettings, LlmConstants } from "@/lib/types";

const getLlmConstants = vi.fn();
vi.mock("@/lib/tauri-api", () => ({
  getLlmConstants: () => getLlmConstants(),
  getSettings: () => Promise.resolve(undefined),
  listProfiles: () => Promise.resolve([]),
  listArbitreProfiles: () => Promise.resolve([]),
  saveSettings: () => Promise.resolve(),
}));

const { availableModels, describeActiveModel, needsOllama, parseModelList, useSettingsStore } = await import("./useSettingsStore");

const base = (): AppSettings => ({ ...useSettingsStore.getState().settings });

const constants: LlmConstants = {
  deepseekDefaultModel: "deepseek-flash",
  deepseekKnownModels: ["deepseek-flash", "deepseek-v4-pro"],
  deepseekDefaultContextBudget: 32768,
  deepseekMinContextBudget: 4096,
  deepseekMaxContextBudget: 262144,
  deepseekMaxNumPredictUi: 16384,
  deepseekPricingDate: "2026-09-10",
  deepseekPricing: [],
  deepseekOffpeakFactor: 0.5,
  deepseekPeakWindowsUtc: [[1, 4], [6, 10]],
  deepseekTopPMinThinking: 0.95,
  budgetWarnRatio: 0.8,
  openaiCompatDefaultBaseUrl: "http://localhost:1234/v1",
  releasesUrl: "https://example.org/releases",
};

beforeEach(() => {
  useSettingsStore.setState({ llmConstants: null });
  getLlmConstants.mockReset();
});

describe("describeActiveModel", () => {
  it("labels the model with its provider", () => {
    expect(describeActiveModel({ ...base(), llmProvider: "ollama", ollamaModel: "llama3.2" })).toBe("ollama · llama3.2");
    expect(describeActiveModel({ ...base(), llmProvider: "deepseek", deepseekModel: "deepseek-flash", ollamaModel: "llama3.2" })).toBe("deepseek · deepseek-flash");
    expect(describeActiveModel({ ...base(), llmProvider: "deepseek", deepseekModel: "" })).toBe("deepseek");
    expect(describeActiveModel({ ...base(), llmProvider: "openaiCompat", openaiCompatModel: "qwen" })).toBe("openaiCompat · qwen");
  });

  it("says \"mixte\" when speakers override the model (v1.20)", () => {
    const s = { ...base(), llmProvider: "ollama" as const, ollamaModel: "llama3.2" };
    expect(describeActiveModel(s, [undefined, "llama3.2", " "])).toBe("ollama · llama3.2");
    expect(describeActiveModel(s, ["mistral", undefined, "mistral"])).toBe("ollama · mixte (llama3.2, mistral)");
  });

  it("lists the models a speaker may pick, global first, per provider", () => {
    const ollama = { ...base(), llmProvider: "ollama" as const, ollamaModel: "llama3.2" };
    expect(availableModels(ollama, [{ name: "mistral" }, { name: "llama3.2" }], constants)).toEqual(["llama3.2", "mistral"]);
    const deepseek = { ...base(), llmProvider: "deepseek" as const, deepseekModel: "deepseek-v4-pro" };
    expect(availableModels(deepseek, [], constants)).toEqual(["deepseek-v4-pro", "deepseek-flash"]);
    const compat = { ...base(), llmProvider: "openaiCompat" as const, openaiCompatModel: "a", openaiCompatModels: JSON.stringify(["b", "", 3, "a"]) };
    expect(availableModels(compat, [], null)).toEqual(["a", "b"]);
    expect(parseModelList("not json")).toEqual([]);
  });
});

describe("needsOllama", () => {
  it("is required for the local provider or when embeddings come from Ollama", () => {
    expect(needsOllama({ ...base(), llmProvider: "ollama" })).toBe(true);
    expect(needsOllama({ ...base(), llmProvider: "deepseek", embeddingModel: "" })).toBe(false);
    expect(needsOllama({ ...base(), llmProvider: "deepseek", embeddingModel: "nomic-embed-text" })).toBe(true);
    expect(needsOllama({ ...base(), llmProvider: "deepseek", embeddingModel: "   " })).toBe(false);
  });
});

describe("useSettingsStore provider helpers", () => {
  it("switches the provider and re-anchors numCtx to the new backend", () => {
    useSettingsStore.setState((s) => ({ settings: { ...s.settings, ollamaModel: "llama3.2", numCtx: 1024 }, llmConstants: constants }));
    useSettingsStore.getState().setProvider("deepseek");
    let s = useSettingsStore.getState().settings;
    expect(s.llmProvider).toBe("deepseek");
    expect(s.ollamaModel).toBe("llama3.2");
    expect(s.numCtx).toBe(constants.deepseekMinContextBudget); // clamped into the API bounds

    useSettingsStore.setState({ modelBudgetInfo: { arch: null, vram: null, recommendedNumCtx: 12288, currentNumCtx: null, ollamaVramMb: null, supportsThink: false, warnings: [] } });
    useSettingsStore.getState().setProvider("ollama");
    s = useSettingsStore.getState().settings;
    expect(s.numCtx).toBe(12288); // VRAM recommendation wins back
  });

  it("loads backend constants once and caches them", async () => {
    getLlmConstants.mockResolvedValue(constants);
    const first = await useSettingsStore.getState().loadLlmConstants();
    const second = await useSettingsStore.getState().loadLlmConstants();
    expect(first?.deepseekDefaultModel).toBe("deepseek-flash");
    expect(second).toBe(first);
    expect(getLlmConstants).toHaveBeenCalledTimes(1);
  });

  it("returns null when the backend call fails", async () => {
    getLlmConstants.mockRejectedValue(new Error("boom"));
    expect(await useSettingsStore.getState().loadLlmConstants()).toBeNull();
    expect(useSettingsStore.getState().llmConstants).toBeNull();
  });
});
