import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ArenaEvent, ArgumentMap, Message } from "@/lib/types";

const saveDiscussionHistory = vi.fn((_request: unknown) => Promise.resolve());
const getSettings = vi.fn();
vi.mock("@/lib/tauri-api", () => ({
  saveDiscussionHistory: (request: unknown) => saveDiscussionHistory(request),
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

  it("exposes threshold crossings as store state with a monotonic sequence", () => {
    dispatch({ type: "emotionalThresholdCrossed", data: { speakerId: "g1", axis: "frustration", direction: "high", value: 90 } });
    const first = useArenaStore.getState().lastThresholdCrossed;
    dispatch({ type: "emotionalThresholdCrossed", data: { speakerId: "g1", axis: "frustration", direction: "high", value: 90 } });
    const second = useArenaStore.getState().lastThresholdCrossed;
    expect(first?.axis).toBe("frustration");
    expect(second!.seq).toBeGreaterThan(first!.seq);
  });

  it("saves the discussion with provider, usage, cost and the JSON map on discussionEnded", () => {
    useSetupStore.setState({ topic: "Sujet", discussionLanguage: "fr", discussionMode: "debate" });
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
    expect(req.modelName).toBe("deepseek · deepseek-flash");
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
