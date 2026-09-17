import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ArenaEvent, ArgumentMap, Message } from "@/lib/types";

const saveDiscussionHistory = vi.fn((_request: unknown) => Promise.resolve());
const getSettings = vi.fn();
const reactToMessage = vi.fn((_id: string, _type: string) => Promise.resolve());
vi.mock("@/lib/tauri-api", () => ({
  saveDiscussionHistory: (request: unknown) => saveDiscussionHistory(request),
  reactToMessage: (id: string, type: string) => reactToMessage(id, type),
  clearRagStore: () => Promise.resolve(),
  getSettings: () => getSettings(),
  listProfiles: () => Promise.resolve([]),
  listArbitreProfiles: () => Promise.resolve([]),
}));

const { parseStreamKey, reasoningStreamKey, useArenaStore } = await import("./useArenaStore");
const { useSetupStore } = await import("./useSetupStore");
const { useSettingsStore } = await import("./useSettingsStore");

const dispatch = (event: ArenaEvent) => useArenaStore.getState().handleEvent(event);

const message = (id: string, speakerId: string): Message => ({
  id,
  discussionId: "d1",
  turnNumber: 1,
  speakerId,
  speakerName: speakerId,
  role: "GladIAteur",
  content: `content ${id}`,
  innerThought: null,
  reactions: [],
  isBanNotification: false,
  timestamp: new Date().toISOString(),
});

const sampleMap: ArgumentMap = {
  theses: [{ id: "t-0", label: "Thèse", speakerId: "g1", speakerName: "Alice", arguments: [] }],
};

beforeEach(() => {
  useArenaStore.getState().reset();
  saveDiscussionHistory.mockClear();
  reactToMessage.mockClear();
  getSettings.mockReset();
  getSettings.mockResolvedValue({ ...useSettingsStore.getState().settings, deepseekPeriodUsageJson: "{\"discussions\":1}" });
});

describe("useArenaStore.handleEvent — v1.16 events", () => {
  it("keeps the latest usage snapshot and budget alert", () => {
    dispatch({
      type: "llmUsageUpdated",
      data: {
        provider: "deepseek", model: "deepseek-flash",
        total: { promptTokens: 1000, cachedTokens: 400, completionTokens: 200, reasoningTokens: 50 },
        calls: 7, estimatedCostUsd: 0.0031, periodSpentUsd: 0.5, budgetUsd: 2, peak: false,
      },
    });
    expect(useArenaStore.getState().llmUsage?.calls).toBe(7);
    expect(useArenaStore.getState().llmUsage?.estimatedCostUsd).toBeCloseTo(0.0031);
    expect(useArenaStore.getState().budgetAlert).toBeNull();

    dispatch({ type: "budgetAlert", data: { level: "warning", spentUsd: 1.6, budgetUsd: 2 } });
    expect(useArenaStore.getState().budgetAlert?.level).toBe("warning");
  });

  it("collects passed speakers per user turn", () => {
    dispatch({ type: "speakerPassed", data: { speakerId: "g1", speakerName: "A" } });
    dispatch({ type: "speakerPassed", data: { speakerId: "g2", speakerName: "B" } });
    expect(useArenaStore.getState().passedSpeakerIds).toEqual(["g1", "g2"]);
    // The next user turn starts a fresh round of decisions
    dispatch({ type: "userTurnReady", data: null });
    expect(useArenaStore.getState().passedSpeakerIds).toEqual([]);
    expect(useArenaStore.getState().userTurnActive).toBe(true);
  });

  it("stores the structured argument map with its novelty markers", () => {
    dispatch({
      type: "argumentMapUpdated",
      data: {
        markdown: "# T", markdownBySpeaker: "# T", thesesCount: 1, argumentsCount: 0,
        map: sampleMap, newNodeIds: ["t-0"], droppedCount: 2,
      },
    });
    const s = useArenaStore.getState();
    expect(s.argumentMap?.theses[0].label).toBe("Thèse");
    expect(s.argumentMapNewNodeIds).toEqual(["t-0"]);
    expect(s.argumentMapDroppedCount).toBe(2);
    expect(s.activityStatus?.type).toBe("argumentMap");
  });

  it("records focus and reasoning level with each directive", () => {
    dispatch({
      type: "directiveGenerated",
      data: {
        speakerId: "g1", speakerName: "A", speechAct: "Challenge", emotionBehavior: null,
        relationshipSummary: "", focusSpeaker: "B", reasoningLevel: "high",
      },
    });
    const d = useArenaStore.getState().directives.get("g1");
    expect(d?.focusSpeaker).toBe("B");
    expect(d?.reasoningLevel).toBe("high");
  });

  it("tracks the act, the scene event of the turn and the timeline of moments", () => {
    dispatch({ type: "actStarted", data: { turn: 1, act: "opening", title: "Ouverture" } });
    dispatch({ type: "turnStarted", data: { turnNumber: 2, speakerOrder: ["g1", "g2"] } });
    dispatch({ type: "sceneEventTriggered", data: { turn: 2, event: { kind: "duel", a: "A", b: "B" }, participants: ["A", "B"] } });
    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    dispatch({ type: "coalitionFormed", data: { turn: 2, a: "g1", b: "g2", aName: "A", bName: "B" } });
    dispatch({ type: "banIssued", data: { bannedId: "g2", bannedName: "B", reason: "r", duration: 1 } });
    let s = useArenaStore.getState();
    expect(s.currentAct).toEqual({ key: "opening", title: "Ouverture", turn: 1 });
    expect(s.sceneEvent?.event.kind).toBe("duel");
    expect(s.timeline.map((t) => t.kind)).toEqual(["act", "sceneEvent", "coalition", "ban"]);
    expect(s.coalition).toEqual({ a: "g1", b: "g2" });
    expect(s.banner?.kind).toBe("ban");
    expect(s.timeline[2]).toMatchObject({ label: "A + B", messageIndex: 1, speakerId: "g1" });
    expect(s.timeline[3]).toMatchObject({ turn: 2, label: "B" });
    // A new turn clears the scene event; the report carries the timeline
    dispatch({ type: "turnStarted", data: { turnNumber: 3, speakerOrder: ["g1"] } });
    s = useArenaStore.getState();
    expect(s.sceneEvent).toBeNull();
    expect(s.coalition).toBeNull();
    expect(s.banner).toMatchObject({ kind: "turn", text: "3" });
    useArenaStore.getState().dismissBanner(s.banner!.id);
    expect(useArenaStore.getState().banner).toBeNull();
    expect(s.buildReport().timeline).toHaveLength(4);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().timeline).toEqual([]);
    expect(useArenaStore.getState().currentAct).toBeNull();
  });

  it("queues floating reactions on the stage, capped, and dismisses them", () => {
    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    for (let i = 0; i < 8; i++) {
      dispatch({ type: "reactionEmitted", data: { messageId: "m1", reaction: { fromSpeakerId: "g2", fromSpeakerName: "B", reactionType: "like", targetMessageId: "m1" } } });
    }
    let s = useArenaStore.getState();
    expect(s.bursts).toHaveLength(6);
    expect(s.bursts[0]).toMatchObject({ from: "g2", to: "g1", reactionType: "like" });
    useArenaStore.getState().dismissBurst(s.bursts[0].id);
    s = useArenaStore.getState();
    expect(s.bursts).toHaveLength(5);
    // Unknown target message: no burst
    dispatch({ type: "reactionEmitted", data: { messageId: "nope", reaction: { fromSpeakerId: "g2", fromSpeakerName: "B", reactionType: "like", targetMessageId: "nope" } } });
    expect(useArenaStore.getState().bursts).toHaveLength(5);
  });

  it("counts concessions from the intentions and hands the awards to the report", () => {
    dispatch({ type: "discussionStarted", data: { discussionId: "d9" } });
    dispatch({ type: "messageComplete", data: { message: { ...message("m1", "g1"), speakerName: "Alice" } } });
    dispatch({ type: "reactionEmitted", data: { messageId: "m1", reaction: { fromSpeakerId: "g2", fromSpeakerName: "Bob", reactionType: "insightful", targetMessageId: "m1" } } });
    dispatch({ type: "intentionGenerated", data: { speakerId: "g1", speakerName: "Alice", target: "Bob", goal: "concede", angle: "", concession: "un point", question: null } });
    dispatch({ type: "intentionGenerated", data: { speakerId: "g1", speakerName: "Alice", target: "Bob", goal: "contest", angle: "", concession: null, question: null } });
    expect(useArenaStore.getState().concessions).toEqual({ g1: 1 });
    const report = useArenaStore.getState().buildReport();
    expect(report.awards.map((a) => a.key)).toEqual(["bestArgument"]);
    expect(report.awards[0]).toMatchObject({ speakerId: "g1", speakerName: "Alice" });
  });

  it("tracks roles, the audience vote window and the mode outcome", () => {
    dispatch({ type: "rolesAssigned", data: { turn: 1, roles: [{ speakerId: "g1", role: "prosecutor", label: "Accusation" }, { speakerId: "g2", role: "defense", label: "Défense" }] } });
    expect(useArenaStore.getState().roles.get("g2")?.label).toBe("Défense");
    dispatch({ type: "audienceVoteRequested", data: { phase: "before", timeoutSecs: 30 } });
    expect(useArenaStore.getState().voteRequest).toEqual({ phase: "before", timeoutSecs: 30 });
    // Nobody voted: the first turn closes the window
    dispatch({ type: "turnStarted", data: { turnNumber: 1, speakerOrder: ["g1", "g2"] } });
    expect(useArenaStore.getState().voteRequest).toBeNull();
    dispatch({ type: "audienceVoteRequested", data: { phase: "after", timeoutSecs: 30 } });
    dispatch({ type: "audienceVoteRecorded", data: { phase: "after", choice: "against" } });
    expect(useArenaStore.getState().voteRequest).toBeNull();
    expect(useArenaStore.getState().votes).toEqual({ after: "against" });
    const outcome = { kind: "audienceSwing" as const, before: null, after: "against", winner: null };
    dispatch({ type: "outcomeReady", data: { outcome } });
    expect(useArenaStore.getState().buildReport().outcome).toEqual(outcome);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().outcome).toBeNull();
    expect(useArenaStore.getState().roles.size).toBe(0);
  });

  it("keeps the step-mode cue until the speaker starts (v1.20.1)", () => {
    dispatch({ type: "awaitingCue", data: { speakerId: "g1", speakerName: "Alice" } });
    expect(useArenaStore.getState().awaitingCue).toEqual({ speakerId: "g1", speakerName: "Alice" });
    dispatch({ type: "speakerActive", data: { speakerId: "g1" } });
    expect(useArenaStore.getState().awaitingCue).toBeNull();
  });

  it("collects the persona recaps for the long memory", () => {
    const recap = { speakerId: "g1", speakerName: "Alice", profileId: "scientist", recap: { positions: ["p"], bestLines: [], allies: [], rivals: ["Bob"], lesson: "l" } };
    dispatch({ type: "personaRecapReady", data: { recap } });
    dispatch({ type: "personaRecapReady", data: { recap: { ...recap, speakerId: "g2", profileId: "lawyer" } } });
    expect(useArenaStore.getState().recaps.map((r) => r.profileId)).toEqual(["scientist", "lawyer"]);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().recaps).toEqual([]);
  });

  it("keeps the unveiled agendas for the report", () => {
    const agendas = [{ speakerId: "g1", speakerName: "Alice", objective: "faire admettre le coût", redLine: "jamais", victory: "Bob cite mes chiffres", achieved: true }];
    dispatch({ type: "agendaRevealed", data: { agendas } });
    expect(useArenaStore.getState().buildReport().agendas).toEqual(agendas);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().agendas).toEqual([]);
  });

  it("collects turn timings and the final diagnostics into the report", () => {
    dispatch({ type: "turnTimings", data: { timings: { turn: 1, phases: [{ name: "speakers", ms: 1200 }, { name: "endOfTurn", ms: 800 }] } } });
    dispatch({ type: "turnTimings", data: { timings: { turn: 2, phases: [{ name: "speakers", ms: 900 }] } } });
    dispatch({ type: "diagnosticsReady", data: { diagnostics: { jsonParseFailures: { memory: 1 }, refusals: 0, retries: 2, intentionCompliance: 0.75 } } });
    const s = useArenaStore.getState();
    expect(s.timings.map((t) => t.turn)).toEqual([1, 2]);
    expect(s.diagnostics?.retries).toBe(2);
    const report = s.buildReport();
    expect(report.timings).toHaveLength(2);
    expect(report.diagnostics?.intentionCompliance).toBe(0.75);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().timings).toEqual([]);
    expect(useArenaStore.getState().diagnostics).toBeNull();
  });

  it("keeps the room mood and passes relationship scores and trends through", () => {
    dispatch({ type: "roomMoodUpdated", data: { avg: { engagement: 50, accord: 50, confiance: 50, frustration: 70, curiosite: 50, enthousiasme: 50 }, label: "tense" } });
    expect(useArenaStore.getState().roomMood?.label).toBe("tense");
    dispatch({ type: "relationshipsUpdated", data: { edges: [{ a: "g1", b: "g2", abLikes: 0, abDislikes: 2, baLikes: 0, baDislikes: 2, kind: "rival", score: -3.4, trend: "cooling" }] } });
    expect(useArenaStore.getState().relationships[0]).toMatchObject({ kind: "rival", score: -3.4, trend: "cooling" });
    // A shift event is accepted (timeline consumer comes with the stage)
    dispatch({ type: "relationshipShift", data: { a: "g1", b: "g2", from: "rival", to: "none" } });
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().roomMood).toBeNull();
  });

  it("keeps the latest intention per speaker and the positions with their trajectory in the report", () => {
    dispatch({
      type: "intentionGenerated",
      data: { speakerId: "g1", speakerName: "A", target: "B", goal: "contest", angle: "les chiffres", concession: null, question: "Et le coût ?" },
    });
    dispatch({
      type: "intentionGenerated",
      data: { speakerId: "g1", speakerName: "A", target: null, goal: "relaunch", angle: "", concession: "le rythme", question: null },
    });
    const i = useArenaStore.getState().intentions.get("g1");
    expect(i).toEqual({ target: null, goal: "relaunch", angle: "", concession: "le rythme", question: null });

    dispatch({
      type: "positionsUpdated",
      data: { positions: [
        { participantName: "A", stance: "ouvert", initialStance: "prudent", shift: "s'est ouvert" },
        { participantName: "B", stance: "critique" },
      ] },
    });
    const positions = useArenaStore.getState().positions;
    expect(positions).toEqual([
      { participantName: "A", stance: "ouvert", initialStance: "prudent", shift: "s'est ouvert", wouldChangeIf: null },
      { participantName: "B", stance: "critique", initialStance: null, shift: null, wouldChangeIf: null },
    ]);
    expect(useArenaStore.getState().buildReport().positions).toEqual(positions);
    useArenaStore.getState().reset();
    expect(useArenaStore.getState().intentions.size).toBe(0);
    expect(useArenaStore.getState().positions).toEqual([]);
  });

  it("exposes threshold crossings as store state with a monotonic sequence", () => {
    dispatch({ type: "emotionalThresholdCrossed", data: { speakerId: "g1", axis: "frustration", direction: "high", value: 90 } });
    const first = useArenaStore.getState().lastThresholdCrossed;
    dispatch({ type: "emotionalThresholdCrossed", data: { speakerId: "g1", axis: "frustration", direction: "high", value: 90 } });
    const second = useArenaStore.getState().lastThresholdCrossed;
    expect(first?.axis).toBe("frustration");
    expect(second!.seq).toBeGreaterThan(first!.seq);
  });

  it("saves the discussion with provider, usage, cost and the JSON map on discussionEnded", () => {
    useSetupStore.setState({
      topic: "Sujet", discussionLanguage: "fr", discussionMode: "debate",
      // One speaker overrides the model: the saved name says so (v1.20)
      gladiateurs: [{ id: "g1", name: "Alice", interventionNumber: 1, systemPrompt: "p", llmParams: { temperature: 0.8, topP: 0.9, topK: 40, numPredict: 1024, numCtx: 8192, repeatPenalty: 1.3 }, model: "deepseek-reasoner" }],
    });
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, llmProvider: "deepseek", deepseekModel: "deepseek-flash", username: "Léo" },
    }));
    dispatch({ type: "discussionStarted", data: { discussionId: "d1" } });
    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    dispatch({
      type: "argumentMapUpdated",
      data: { markdown: "# T", markdownBySpeaker: "# T", thesesCount: 1, argumentsCount: 0, map: sampleMap, newNodeIds: [], droppedCount: 0 },
    });
    dispatch({
      type: "llmUsageUpdated",
      data: {
        provider: "deepseek", model: "deepseek-flash",
        total: { promptTokens: 10, cachedTokens: 0, completionTokens: 5, reasoningTokens: 0 },
        calls: 1, estimatedCostUsd: 0.01, periodSpentUsd: 0, budgetUsd: 0, peak: true,
      },
    });
    dispatch({ type: "discussionEnded", data: null });

    expect(saveDiscussionHistory).toHaveBeenCalledTimes(1);
    const req = saveDiscussionHistory.mock.calls[0][0] as unknown as Record<string, unknown>;
    expect(req.modelName).toBe("deepseek · mixte (deepseek-flash, deepseek-reasoner)");
    expect(req.llmProvider).toBe("deepseek");
    expect(req.estimatedCostUsd).toBe(0.01);
    expect((req.usage as { total: { promptTokens: number } }).total.promptTokens).toBe(10);
    expect(JSON.parse(req.argumentMapJson as string)).toEqual(sampleMap);
    expect(useArenaStore.getState().status).toBe("ended");
  });

  it("does not save an empty discussion and reset() clears the new fields", () => {
    dispatch({ type: "discussionStarted", data: { discussionId: "d2" } });
    dispatch({ type: "discussionEnded", data: null });
    expect(saveDiscussionHistory).not.toHaveBeenCalled();

    dispatch({ type: "speakerPassed", data: { speakerId: "g1", speakerName: "A" } });
    dispatch({ type: "budgetAlert", data: { level: "exceeded", spentUsd: 2, budgetUsd: 2 } });
    useArenaStore.getState().reset();
    const s = useArenaStore.getState();
    expect(s.passedSpeakerIds).toEqual([]);
    expect(s.budgetAlert).toBeNull();
    expect(s.llmUsage).toBeNull();
    expect(s.argumentMap).toBeNull();
    expect(s.lastThresholdCrossed).toBeNull();
  });

  it("re-hydrates the settings store once the discussion ended", async () => {
    dispatch({ type: "discussionStarted", data: { discussionId: "d3" } });
    dispatch({ type: "discussionEnded", data: null });
    await vi.waitFor(() => expect(getSettings).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(useSettingsStore.getState().settings.deepseekPeriodUsageJson).toBe("{\"discussions\":1}"));
  });

  it("streams live reasoning under a separate buffer key and clears it with the message", () => {
    const pushed: [string, string][] = [];
    const cleared: string[] = [];
    useArenaStore.getState().registerStreamBuffer({
      pushToken: (id, token) => pushed.push([id, token]),
      clearSpeaker: (id) => cleared.push(id),
      clearAll: () => {},
    });
    dispatch({ type: "thoughtChunk", data: { speakerId: "g1", chunk: "Je pense" } });
    dispatch({ type: "messageChunk", data: { speakerId: "g1", chunk: "Bonjour" } });
    expect(pushed).toEqual([[reasoningStreamKey("g1"), "Je pense"], ["g1", "Bonjour"]]);
    expect(parseStreamKey(reasoningStreamKey("g1"))).toEqual({ speakerId: "g1", kind: "reasoning" });
    expect(parseStreamKey("g1")).toEqual({ speakerId: "g1", kind: "content" });

    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    expect(cleared).toEqual(["g1", reasoningStreamKey("g1")]);
    useArenaStore.getState().unregisterStreamBuffer();
  });

  it("never throws on a malformed event payload", () => {
    expect(() => dispatch({ type: "messageComplete", data: { message: undefined as unknown as Message } })).not.toThrow();
    expect(useArenaStore.getState().messages).toHaveLength(0);
  });
});

describe("audience reactions", () => {
  it("shows the reaction at once and lets the engine confirm it", async () => {
    dispatch({ type: "discussionStarted", data: { discussionId: "d9" } });
    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    const pendingCall = useArenaStore.getState().reactAsAudience("m1", "laugh");
    let r = useArenaStore.getState().messages[0].reactions;
    expect(r).toHaveLength(1);
    expect(r[0].pending).toBe(true);
    await pendingCall;
    dispatch({ type: "reactionEmitted", data: { messageId: "m1", reaction: { fromSpeakerId: "user", fromSpeakerName: "Léo", reactionType: "laugh", targetMessageId: "m1" } } });
    r = useArenaStore.getState().messages[0].reactions;
    expect(r).toHaveLength(1);
    expect(r[0].pending).toBeUndefined();
    expect(r[0].fromSpeakerName).toBe("Léo");
    // A peer reaction of the same type still appends
    dispatch({ type: "reactionEmitted", data: { messageId: "m1", reaction: { fromSpeakerId: "g2", fromSpeakerName: "B", reactionType: "laugh", targetMessageId: "m1" } } });
    expect(useArenaStore.getState().messages[0].reactions).toHaveLength(2);
  });

  it("withdraws the optimistic reaction when the command fails", async () => {
    reactToMessage.mockRejectedValueOnce({ kind: "NoActiveDiscussion", message: "No active discussion" });
    dispatch({ type: "discussionStarted", data: { discussionId: "d10" } });
    dispatch({ type: "messageComplete", data: { message: message("m1", "g1") } });
    await useArenaStore.getState().reactAsAudience("m1", "like");
    expect(useArenaStore.getState().messages[0].reactions).toHaveLength(0);
  });
});
