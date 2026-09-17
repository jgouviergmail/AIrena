import { beforeEach, describe, expect, it } from "vitest";

// Minimal storage so the preferences survive a "reload" (node has no localStorage)
const memory = new Map<string, string>();
(globalThis as { localStorage?: Storage }).localStorage = {
  getItem: (k: string) => memory.get(k) ?? null,
  setItem: (k: string, v: string) => void memory.set(k, v),
  removeItem: (k: string) => void memory.delete(k),
  clear: () => memory.clear(),
  key: () => null,
  length: 0,
} as Storage;

const { readPrefs, useUiStore } = await import("./useUiStore");

describe("useUiStore", () => {
  beforeEach(() => {
    memory.clear();
    useUiStore.setState({ stageVisible: true, presentationMode: false, viewedTurn: null });
  });

  it("persists the stage preference but never the projection mode", () => {
    useUiStore.getState().toggleStage();
    useUiStore.getState().setPresentationMode(true);
    expect(JSON.parse(memory.get("airena.ui") ?? "{}")).toEqual({ stageVisible: false });
    expect(useUiStore.getState().presentationMode).toBe(true);
  });

  it("ignores a malformed or partial saved preference", () => {
    memory.set("airena.ui", "{not json");
    expect(readPrefs()).toEqual({ stageVisible: true });
    memory.set("airena.ui", JSON.stringify({ stageVisible: "yes" }));
    expect(readPrefs()).toEqual({ stageVisible: true });
  });
});
