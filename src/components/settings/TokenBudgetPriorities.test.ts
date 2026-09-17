import { describe, expect, it } from "vitest";
import { moveSection, parsePriorities } from "./TokenBudgetPriorities";
import { CONFIGURABLE_BUDGET_SECTIONS } from "@/lib/types";

describe("parsePriorities", () => {
  it("returns the default order (ranks from 4) for empty or invalid JSON", () => {
    for (const raw of ["", "{", "[]", JSON.stringify([{ section: "immediateMemory", rank: 4, floor: 0, ceiling: 0 }])]) {
      const p = parsePriorities(raw);
      expect(p.map((s) => s.section)).toEqual(CONFIGURABLE_BUDGET_SECTIONS);
      expect(p[0].rank).toBe(4);
    }
  });

  it("keeps a complete stored order and ignores backend-managed sections", () => {
    const stored = [
      ...CONFIGURABLE_BUDGET_SECTIONS.map((section, i) => ({ section, rank: 10 - i, floor: 0, ceiling: 0 })),
      { section: "fullDocument", rank: 1, floor: 0, ceiling: 0 },
    ];
    const p = parsePriorities(JSON.stringify(stored));
    expect(p).toHaveLength(CONFIGURABLE_BUDGET_SECTIONS.length);
    expect(p[0].section).toBe(CONFIGURABLE_BUDGET_SECTIONS[CONFIGURABLE_BUDGET_SECTIONS.length - 1]);
    expect(p.some((s) => s.section === "fullDocument")).toBe(false);
  });

  it("appends the sections a v1.16 order predates, after the user's order", () => {
    const v116 = ["positionalMap", "currentTurnMessages", "immediateMemory", "contextualSummary", "cognitiveDirectives", "arbitreDirectives", "webWikiSearch"]
      .map((section, i) => ({ section, rank: i + 4, floor: 0, ceiling: 0 }));
    const p = parsePriorities(JSON.stringify(v116));
    expect(p.map((s) => s.section)).toEqual([...v116.map((s) => s.section), "openLoops", "debateState"]);
    expect(p.map((s) => s.rank)).toEqual(p.map((_, i) => i + 4));
  });
});

describe("moveSection", () => {
  it("swaps neighbours and renumbers ranks; out-of-range moves are no-ops", () => {
    const base = parsePriorities("");
    const moved = moveSection(base, 1, 0);
    expect(moved[0].section).toBe(base[1].section);
    expect(moved.map((s) => s.rank)).toEqual(base.map((_, i) => i + 4));
    expect(moveSection(base, 0, -1)).toBe(base);
    expect(moveSection(base, 2, 2)).toBe(base);
  });
});
