import { describe, expect, it } from "vitest";
import { collectTags, EMPTY_FILTERS, filterDiscussions, hasActiveFilters, normaliseTag } from "./history-filters";
import type { DiscussionSummary } from "./types";

const disc = (id: string, over: Partial<DiscussionSummary>): DiscussionSummary => ({
  id, topic: id, discussionLanguage: "fr", modelName: "m", participants: [], totalTurns: 1, hasSynthesis: true, createdAt: "",
  discussionMode: "debate", documentFormat: "none", hasArgumentMap: false, llmProvider: "ollama", totalTokens: 0, estimatedCostUsd: 0,
  tags: [], favorite: false, ...over,
});
const list = [
  disc("a", { discussionMode: "trial", llmProvider: "deepseek", participants: [{ id: "1", name: "Le Scientifique", role: "GladIAteur", emoji: "🔬" }], tags: ["ia", "travail"], favorite: true }),
  disc("b", { tags: ["ia"] }),
  disc("c", { participants: [{ id: "2", name: "La Juriste", role: "GladIAteur", emoji: "⚖️" }] }),
];

describe("history filters", () => {
  it("narrows by mode, provider, participant, tag and favourite", () => {
    expect(filterDiscussions(list, EMPTY_FILTERS).map((d) => d.id)).toEqual(["a", "b", "c"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, mode: "trial" }).map((d) => d.id)).toEqual(["a"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, provider: "ollama" }).map((d) => d.id)).toEqual(["b", "c"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, participant: "  scienti " }).map((d) => d.id)).toEqual(["a"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, tag: "ia" }).map((d) => d.id)).toEqual(["a", "b"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, favoriteOnly: true }).map((d) => d.id)).toEqual(["a"]);
    expect(filterDiscussions(list, { ...EMPTY_FILTERS, tag: "ia", provider: "ollama" }).map((d) => d.id)).toEqual(["b"]);
    expect(hasActiveFilters(EMPTY_FILTERS)).toBe(false);
    expect(hasActiveFilters({ ...EMPTY_FILTERS, participant: "x" })).toBe(true);
  });

  it("collects tags by frequency and normalises typed ones", () => {
    expect(collectTags(list)).toEqual(["ia", "travail"]);
    expect(normaliseTag("  #Travail  ")).toBe("travail");
    expect(normaliseTag("x".repeat(40))).toHaveLength(24);
  });
});
