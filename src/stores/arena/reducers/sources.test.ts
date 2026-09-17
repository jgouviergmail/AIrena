import { describe, expect, it } from "vitest";
import { initialArenaData } from "../types";
import { sourcesReducer } from "./sources";
import type { ArenaData } from "../types";
import type { Message } from "@/lib/types";

const msg = (id: string, speakerId: string, content: string): Message => ({
  id, discussionId: "d", turnNumber: 2, speakerId, speakerName: speakerId, role: "GladIAteur", content,
  innerThought: null, reactions: [], isBanNotification: false, timestamp: "2026-09-16T10:00:00Z",
});

function apply(state: ArenaData, patch: Partial<ArenaData> | null): ArenaData {
  return patch ? { ...state, ...patch } : state;
}

describe("sourcesReducer", () => {
  it("collects web, wiki and rag sources for the active speaker and attaches them to the completed message", () => {
    let s = { ...initialArenaData(), currentTurn: 2 };
    s = apply(s, sourcesReducer(s, { type: "speakerActive", data: { speakerId: "g1" } }));
    s = apply(s, sourcesReducer(s, {
      type: "webSearchPerformed",
      data: { speakerId: "g1", speakerName: "Le Scientifique", queries: ["q"], resultsCount: 1, poolUsed: 1,
        results: [{ title: "Étude", url: "https://www.etude.org/x", domain: "www.etude.org", snippet: "les métiers techniques se transforment" }] },
    }));
    s = apply(s, sourcesReducer(s, {
      type: "wikiSearchPerformed",
      data: { speakerId: "g1", speakerName: "Le Scientifique", queries: ["Automatisation"], resultsCount: 1, poolUsed: 1,
        articleUrls: ["https://fr.wikipedia.org/wiki/Automatisation"], articles: [{ title: "Automatisation", url: "https://fr.wikipedia.org/wiki/Automatisation", snippet: "" }] },
    }));
    s = apply(s, sourcesReducer(s, {
      type: "ragContextInjected",
      data: { speakerId: "g1", speakerName: "Le Scientifique", chunks: [{ fileName: "notes.md", chunkIndex: 0, preview: "p", relevanceScore: 1 }], cached: true },
    }));
    expect(s._pendingSources).toHaveLength(3);
    expect(s.sources).toHaveLength(0);

    s = apply(s, sourcesReducer(s, { type: "messageComplete", data: { message: msg("m1", "g1", "Comme le dit etude.org, tout change.") } }));
    expect(s._pendingSources).toHaveLength(0);
    expect(s.sources.map((x) => [x.kind, x.messageId, x.turn, x.cited])).toEqual([
      ["web", "m1", 2, true],
      ["wiki", "m1", 2, false],
      ["rag", "m1", 2, false],
    ]);
  });

  it("drops pending sources when the next speaker starts and ignores events without sources", () => {
    let s = initialArenaData();
    s = apply(s, sourcesReducer(s, {
      type: "webSearchPerformed",
      data: { speakerId: "g1", speakerName: "A", queries: ["q"], resultsCount: 0, poolUsed: 1, results: [{ title: "T", url: "https://t", domain: "t", snippet: "" }] },
    }));
    expect(s._pendingSources).toHaveLength(1);
    s = apply(s, sourcesReducer(s, { type: "speakerActive", data: { speakerId: "g2" } }));
    expect(s._pendingSources).toHaveLength(0);
    expect(sourcesReducer(s, { type: "messageComplete", data: { message: msg("m2", "g2", "x") } })).toBeNull();
    expect(sourcesReducer(s, { type: "webSearchPerformed", data: { speakerId: "g2", speakerName: "B", queries: [], resultsCount: 0, poolUsed: 0, results: [] } })).toBeNull();
  });
});
