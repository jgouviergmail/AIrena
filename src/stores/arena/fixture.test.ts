// Replays the full-feature event stream recorded by the Rust engine
// (`cargo test --lib export_event_fixture -- --ignored`) through the store:
// the reducers must accept every event the backend really emits.
import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../../src-tauri/fixtures/events-full.json";
import type { ArenaEvent } from "@/lib/types";
import { parseReportJson } from "@/lib/report";

const saveDiscussionHistory = vi.fn((_request: unknown) => Promise.resolve());
vi.mock("@/lib/tauri-api", () => ({
  saveDiscussionHistory: (request: unknown) => saveDiscussionHistory(request),
  clearRagStore: () => Promise.resolve(),
  getSettings: () => Promise.resolve(useSettingsStore.getState().settings),
  listProfiles: () => Promise.resolve([]),
  listArbitreProfiles: () => Promise.resolve([]),
}));

const { useArenaStore } = await import("../useArenaStore");
const { useSetupStore } = await import("../useSetupStore");
const { useSettingsStore } = await import("../useSettingsStore");

const events = fixture as unknown as ArenaEvent[];

beforeEach(() => {
  useArenaStore.getState().reset();
  saveDiscussionHistory.mockClear();
  useSetupStore.setState({
    topic: "L'IA va-t-elle remplacer les développeurs ?",
    gladiateurs: [
      { id: "g1", name: "Le Scientifique", interventionNumber: 1, systemPrompt: "", llmParams: useSetupStore.getState().arbitre.llmParams },
      { id: "g2", name: "Le Philosophe", interventionNumber: 2, systemPrompt: "", llmParams: useSetupStore.getState().arbitre.llmParams },
      { id: "g3", name: "La Juriste", interventionNumber: 3, systemPrompt: "", llmParams: useSetupStore.getState().arbitre.llmParams },
    ],
    arbitre: { ...useSetupStore.getState().arbitre, id: "arb", name: "Le Modérateur" },
  });
});

describe("full-feature fixture replay", () => {
  it("rebuilds the whole discussion state and persists a report", () => {
    const counts = new Map<string, number>();
    for (const e of events) {
      counts.set(e.type, (counts.get(e.type) ?? 0) + 1);
      useArenaStore.getState().handleEvent(e);
    }
    const s = useArenaStore.getState();

    expect(s.status).toBe("ended");
    expect(s.currentTurn).toBe(6);
    expect(s.messages).toHaveLength(counts.get("messageComplete") ?? -1);
    expect(s.error).toBeNull();
    // Reactions land on their target message
    const attached = s.messages.reduce((n, m) => n + m.reactions.length, 0);
    expect(attached).toBe(counts.get("reactionEmitted") ?? -1);
    expect(s.relationships.length).toBeGreaterThan(0);
    // Emotions for the moderator + 3 gladiateurs, one snapshot per turn
    expect(s.emotions.size).toBe(4);
    expect(s.emotionHistory.get("g1")).toHaveLength(6);
    expect(s.moodSummary.get("g2")).toBeTruthy();
    // Artifacts
    expect(s.argumentMap?.theses.length).toBeGreaterThan(0);
    expect(s.argumentMapMarkdown).toContain("#");
    expect(s.documentContent).toContain("Point consolidé");
    expect(s.documentFormat).toBe("md");
    expect(s.llmUsage?.model).toBe("deepseek-flash");
    expect(s.llmUsage?.estimatedCostUsd).toBeGreaterThan(0);
    expect(s.synthesis).toContain("Synthèse");
    expect(s.synthesisStreaming).toBe("");
    expect(s.lastThresholdCrossed).not.toBeNull();
    // Backstage (v1.17): one intention per gladiateur, positions with their trajectory
    expect(s.intentions.size).toBe(3);
    expect(s.intentions.get("g1")?.target).toBeTruthy();
    expect(s.positions.length).toBeGreaterThan(0);
    expect(s.positions.some((p) => p.initialStance !== null && p.shift !== null)).toBe(true);
    // Emotion-driven theatre: stage directions in the feed, a room mood, edges with a trend
    expect(s.messages.some((m) => m.kind === "stageDirection")).toBe(true);
    expect(s.roomMood?.label).toBeTruthy();
    expect(s.relationships.every((e) => typeof e.score === "number" && e.trend)).toBe(true);
    // Staging (v1.18): the debate script ends on the closing act, moments are on the timeline
    expect(s.currentAct?.key).toBe("closingStatements");
    expect(s.timeline.filter((t) => t.kind === "act").length).toBeGreaterThanOrEqual(3);
    expect(s.messages.some((m) => m.kind === "actAnnouncement")).toBe(true);
    // Measures: one timings row per turn, diagnostics before the end
    expect(s.timings.map((t) => t.turn)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(s.diagnostics).not.toBeNull();
    // Secret agendas unveiled after the synthesis, with the moderator verdicts (v1.19)
    expect(s.agendas.map((a) => [a.speakerId, a.achieved])).toEqual([["g1", true], ["g2", false], ["g3", null]]);
    expect(s.agendas[0].objective).toContain("faire admettre le coût");

    // Persisted once, with the report carrying the emotion history and relationships
    expect(saveDiscussionHistory).toHaveBeenCalledTimes(1);
    const req = saveDiscussionHistory.mock.calls[0][0] as { reportJson: string; totalTurns: number; messages: unknown[] };
    expect(req.totalTurns).toBe(6);
    expect(req.messages).toHaveLength(s.messages.length);
    const report = parseReportJson(req.reportJson);
    expect(Object.keys(report.emotionHistory)).toHaveLength(4);
    expect(report.relationships).toEqual(s.relationships);
    expect(report.positions).toEqual(s.positions);
    expect(report.agendas).toEqual(s.agendas);
    expect(report.timings).toHaveLength(6);
    expect(report.diagnostics).toEqual(s.diagnostics);
    // Closing credits computed from the replayed state (reactions were exchanged)
    expect(report.awards.some((a) => a.key === "bestArgument")).toBe(true);
    expect(report.timeline.length).toBeGreaterThan(0);
  });

  it("reset() returns to the pristine state with fresh maps", () => {
    for (const e of events) useArenaStore.getState().handleEvent(e);
    useArenaStore.getState().reset();
    const s = useArenaStore.getState();
    expect(s.status).toBe("idle");
    expect(s.messages).toEqual([]);
    expect(s.emotions.size).toBe(0);
    expect(s.emotionHistory.size).toBe(0);
    expect(s.argumentMap).toBeNull();
    expect(s.documentContent).toBe("");
  });
});
