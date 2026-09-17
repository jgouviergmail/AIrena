import { describe, expect, it } from "vitest";
import { computeAwards, computeScores } from "./score";
import type { ArgumentMap, BanInfo, Message, Reaction } from "./types";

const react = (type: Reaction["reactionType"], from = "g2"): Reaction => ({ fromSpeakerId: from, fromSpeakerName: from, reactionType: type, targetMessageId: "m" });

const msg = (id: string, speakerId: string, turn: number, reactions: Reaction[] = [], kind?: Message["kind"]): Message => ({
  id, discussionId: "d", turnNumber: turn, speakerId, speakerName: speakerId.toUpperCase(), role: "GladIAteur", content: `Contenu ${id} assez long pour un extrait`,
  innerThought: null, reactions, isBanNotification: false, kind, timestamp: new Date().toISOString(),
});

const participants = [{ id: "g1", name: "G1" }, { id: "g2", name: "G2" }, { id: "g3", name: "G3" }];
const map: ArgumentMap = {
  theses: [
    { id: "t1", label: "T", speakerId: "g1", speakerName: "G1", arguments: [{ id: "a1", label: "A", argType: "support", speakerId: "g2", speakerName: "G2", targetsThesisId: "t1", children: [{ id: "a2", label: "B", argType: "counter", speakerId: "g1", speakerName: "G1", targetsThesisId: "t1", children: [] }] }] },
  ],
};

describe("computeScores", () => {
  it("adds reactions, map contributions, concessions and bans, ranks with ties", () => {
    const messages = [
      msg("m1", "g1", 1, [react("like"), react("insightful")]),
      msg("m2", "g2", 1, [react("dislike", "g1"), react("offTopic", "g3")]),
      msg("m3", "g1", 2, [react("laugh")]),
      msg("s1", "g3", 2, [react("like")], "stageDirection"),
    ];
    const bans = new Map<string, BanInfo>([["g2", { remaining: 0, totalBans: 1, justIssued: false }]]);
    const rows = computeScores({ messages, argumentMap: map, bans, concessions: { g3: 1 }, participants });
    const g1 = rows.find((r) => r.speakerId === "g1")!;
    expect(g1.reactions).toBe(2 + 3 + 1);
    expect(g1.byTurn).toEqual({ 1: 5, 2: 1 });
    expect(g1.arguments).toBe(3 + 1);
    expect(g1.points).toBe(10);
    const g2 = rows.find((r) => r.speakerId === "g2")!;
    expect(g2.reactions).toBe(-3);
    expect(g2.arguments).toBe(1);
    expect(g2.bans).toBe(-5);
    expect(g2.points).toBe(-7);
    const g3 = rows.find((r) => r.speakerId === "g3")!;
    expect(g3.reactions).toBe(0);
    expect(g3.concessions).toBe(2);
    expect(rows.map((r) => [r.speakerId, r.rank])).toEqual([["g1", 1], ["g3", 2], ["g2", 3]]);
    // Ties share the rank
    const tied = computeScores({ messages: [], argumentMap: null, bans: new Map(), concessions: {}, participants });
    expect(tied.map((r) => r.rank)).toEqual([1, 1, 1]);
  });
});

describe("computeAwards", () => {
  it("names a winner per award and skips the ones without one", () => {
    const messages = [msg("m1", "g1", 1, [react("insightful"), react("like")]), msg("m2", "g2", 1, [react("dislike", "g1")])];
    const awards = computeAwards({
      messages, argumentMap: map, bans: new Map(), concessions: {}, participants,
      positions: [{ participantName: "G2", stance: "ouvert", initialStance: "fermé", shift: "s'est ouvert", wouldChangeIf: null }],
      timeline: [{ kind: "relationshipShift", turn: 3, messageIndex: 4, label: "rival → none", speakerId: "g2" }],
    });
    const byKey = Object.fromEntries(awards.map((a) => [a.key, a]));
    expect(byKey.bestArgument).toMatchObject({ speakerId: "g1", detail: expect.stringContaining("Contenu m1") });
    expect(byKey.biggestShift).toMatchObject({ speakerId: "g2", detail: "fermé → ouvert" });
    expect(byKey.mostContested).toMatchObject({ speakerId: "g2", detail: "1" });
    expect(byKey.mostProlific).toMatchObject({ speakerId: "g1", detail: "4" });
    expect(byKey.reconciliation).toMatchObject({ speakerId: "g2", speakerName: "G2" });
    expect(computeAwards({ messages: [], argumentMap: null, bans: new Map(), concessions: {}, participants, positions: [], timeline: [] })).toEqual([]);
  });
});
