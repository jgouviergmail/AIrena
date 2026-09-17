import { create } from "zustand";
import { logger } from "@/lib/logger";
import { reactToMessage, saveDiscussionHistory } from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import { useUiStore } from "@/stores/useUiStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { emptyReport, serializeReport, type DiscussionReport } from "@/lib/report";
import { computeAwards } from "@/lib/score";
import { spokenParticipants } from "@/lib/stage";
import { useSetupStore } from "@/stores/useSetupStore";
import { describeActiveModel, useSettingsStore } from "@/stores/useSettingsStore";
import type { ArenaEvent, ParticipantInfo, ReactionType, SpeakerRole, UsageLedger } from "@/lib/types";
import { EMPTY_LEDGER } from "@/lib/types";
import { initialArenaData, type ArenaData, type ArenaReducer } from "./arena/types";
import { reasoningStreamKey, SynthesisBuffer, type StreamBufferCallbacks } from "./arena/streaming";
import { messagesReducer } from "./arena/reducers/messages";
import { knowledgeReducer } from "./arena/reducers/knowledge";
import { emotionsReducer } from "./arena/reducers/emotions";
import { turnReducer } from "./arena/reducers/turn";
import { artifactsReducer } from "./arena/reducers/artifacts";
import { sourcesReducer } from "./arena/reducers/sources";
import { stageReducer } from "./arena/reducers/stage";

export type { ActivityStatus, ArenaData, BudgetAlertInfo, LlmUsageStatus, ThresholdCrossing } from "./arena/types";
export { parseStreamKey, reasoningStreamKey } from "./arena/streaming";

interface ArenaState extends ArenaData {
  handleEvent: (event: ArenaEvent) => void;
  registerStreamBuffer: (cb: StreamBufferCallbacks) => void;
  unregisterStreamBuffer: () => void;
  /** Voice and sounds listen to the raw stream and to the events (v1.18). */
  registerAudioSink: (sink: AudioSink) => void;
  unregisterAudioSink: () => void;
  /** Report built from the collected state (persisted with the discussion). */
  buildReport: () => DiscussionReport;
  /** Audience reaction: shown at once (pending), confirmed or withdrawn by the engine's answer. */
  reactAsAudience: (messageId: string, type: ReactionType) => Promise<void>;
  /** A floating reaction finished its flight. */
  dismissBurst: (id: string) => void;
  /** A banner finished its display. */
  dismissBanner: (id: string) => void;
  reset: () => void;
}

/** Pure reducers, applied in order; each owns a disjoint set of event types. */
const REDUCERS: ArenaReducer[] = [messagesReducer, knowledgeReducer, emotionsReducer, turnReducer, artifactsReducer, sourcesReducer, stageReducer];

// Stream buffer ref — lives outside the store to avoid triggering re-renders
let streamBuffer: StreamBufferCallbacks | null = null;

/** Listener of the audio layer: streamed text and engine events (never re-renders anything). */
export interface AudioSink {
  onChunk: (speakerId: string, chunk: string) => void;
  onEvent: (event: ArenaEvent) => void;
}
let audioSink: AudioSink | null = null;

// Track first messageChunk per speaker to transition activity: "thinking" → "writing" (once)
let writingSpeakerId: string | null = null;

const synthesis = new SynthesisBuffer((chunk) => {
  useArenaStore.setState((s) => ({ synthesisStreaming: s.synthesisStreaming + chunk }));
});

export const useArenaStore = create<ArenaState>((set, get) => ({
  ...initialArenaData(),

  registerStreamBuffer: (cb) => {
    streamBuffer = cb;
  },
  unregisterStreamBuffer: () => {
    streamBuffer = null;
  },
  registerAudioSink: (sink) => {
    audioSink = sink;
  },
  unregisterAudioSink: () => {
    audioSink = null;
  },

  handleEvent: (event: ArenaEvent) => {
    try {
      // Log all events except high-frequency streaming chunks
      if (event.type !== "messageChunk" && event.type !== "thoughtChunk" && event.type !== "synthesisChunk") {
        logger.info("arena", `Event: ${event.type}`, (event as Record<string, unknown>).data);
      }

      if (event.type !== "messageChunk" && event.type !== "thoughtChunk" && event.type !== "synthesisChunk") {
        audioSink?.onEvent(event);
      }

      // Side effects first (buffers, activity labels needing other stores), then the pure reducers
      switch (event.type) {
        case "discussionStarted":
          logger.info("arena", `Discussion started: ${event.data.discussionId}`);
          set({ discussionId: event.data.discussionId, status: "running", error: null });
          useUiStore.setState({ viewedTurn: null });
          break;

        case "messageChunk":
          streamBuffer?.pushToken(event.data.speakerId, event.data.chunk);
          audioSink?.onChunk(event.data.speakerId, event.data.chunk);
          if (writingSpeakerId !== event.data.speakerId) {
            writingSpeakerId = event.data.speakerId;
            set((s) => ({ activityStatus: s.activityStatus ? { ...s.activityStatus, type: "writing" } : null }));
          }
          break;

        case "thoughtChunk":
          // Live reasoning (DeepSeek / Ollama think) — same 60 ms buffer, separate key
          streamBuffer?.pushToken(reasoningStreamKey(event.data.speakerId), event.data.chunk);
          break;

        case "thoughtComplete":
          // The thought is attached to the Message by the backend when MessageComplete arrives;
          // the live bubble stays visible until then.
          break;

        case "messageComplete":
          if (event.data.message?.id) {
            streamBuffer?.clearSpeaker(event.data.message.speakerId);
            streamBuffer?.clearSpeaker(reasoningStreamKey(event.data.message.speakerId));
          }
          break;

        case "speakerActive": {
          streamBuffer?.clearAll();
          writingSpeakerId = null;
          const setup = useSetupStore.getState();
          const speakerName = setup.arbitre.id === event.data.speakerId
            ? setup.arbitre.name
            : setup.gladiateurs.find((g) => g.id === event.data.speakerId)?.name ?? event.data.speakerId;
          set({ activityStatus: { type: "thinking", speakerName }, awaitingCue: null });
          break;
        }

        case "synthesisChunk":
          if (!synthesis.active) set({ status: "synthesizing", activityStatus: { type: "synthesis" } });
          synthesis.push(event.data.chunk);
          break;

        case "synthesisComplete":
          synthesis.stop();
          set({ synthesis: event.data.summary, synthesisStreaming: "", status: "ended", activityStatus: null });
          break;

        case "discussionEnded":
          synthesis.stop();
          persistDiscussion(get());
          // The engine updated server-owned counters (cloud spend period, Tavily
          // credits): refresh the settings store so a later save never writes
          // back the values hydrated before the discussion.
          useSettingsStore.getState().hydrate().catch(() => {});
          set({ status: "ended", determiningOrder: false, activityStatus: null, awaitingCue: null });
          break;

        case "error":
          logger.error("arena", `Engine error: ${event.data.message}`);
          set({ error: event.data.message, determiningOrder: false });
          break;

        default:
          break;
      }

      for (const reduce of REDUCERS) {
        const patch = reduce(get(), event);
        if (patch) set(patch);
      }
    } catch (err) {
      const msg = err instanceof Error ? `${err.message}\n${err.stack}` : String(err);
      logger.error("arena", `handleEvent crash on "${event.type}": ${msg}`);
      console.error("handleEvent crash:", event, err);
    }
  },

  buildReport: () => buildReport(get()),

  dismissBurst: (id) => set((s) => ({ bursts: s.bursts.filter((b) => b.id !== id) })),
  dismissBanner: (id) => set((s) => (s.banner?.id === id ? { banner: null } : {})),

  reactAsAudience: async (messageId, type) => {
    const pending = {
      fromSpeakerId: "user",
      fromSpeakerName: useSettingsStore.getState().settings.username,
      reactionType: type,
      targetMessageId: messageId,
      pending: true,
    };
    const withReaction = (msgs: ArenaData["messages"], fn: (r: ArenaData["messages"][number]["reactions"]) => ArenaData["messages"][number]["reactions"]) =>
      msgs.map((m) => (m.id === messageId ? { ...m, reactions: fn(m.reactions ?? []) } : m));
    set((s) => ({ messages: withReaction(s.messages, (r) => [...r, pending]) }));
    try {
      await reactToMessage(messageId, type);
    } catch (e) {
      // Withdraw the optimistic chip (last pending of this type) and say why
      set((s) => ({
        messages: withReaction(s.messages, (r) => {
          const idx = r.map((x) => x.pending === true && x.reactionType === type).lastIndexOf(true);
          return idx >= 0 ? r.filter((_, i) => i !== idx) : r;
        }),
      }));
      toast.error(extractErrorMessage(e));
    }
  },

  reset: () => {
    streamBuffer?.clearAll();
    synthesis.stop();
    writingSpeakerId = null;
    set(initialArenaData());
  },
}));

/** Report assembled from the collected state. */
function buildReport(s: ArenaData): DiscussionReport {
  return {
    ...emptyReport(),
    awards: computeAwards({
      messages: s.messages,
      argumentMap: s.argumentMap,
      bans: s.bans,
      concessions: s.concessions,
      participants: spokenParticipants(s.messages),
      positions: s.positions,
      timeline: s.timeline,
    }),
    emotionHistory: Object.fromEntries(s.emotionHistory),
    relationships: s.relationships,
    positions: s.positions,
    agendas: s.agendas,
    outcome: s.outcome,
    sources: s.sources,
    timings: s.timings,
    timeline: s.timeline,
    diagnostics: s.diagnostics,
  };
}

/** Auto-save to history (fire-and-forget). Skipped when nothing was said. */
function persistDiscussion(arena: ArenaData) {
  if (!arena.discussionId || arena.messages.length === 0) return;
  const setupState = useSetupStore.getState();
  const settingsState = useSettingsStore.getState();

  const participants: ParticipantInfo[] = [
    { id: setupState.arbitre.id, name: setupState.arbitre.name, role: "IArbitre" as SpeakerRole, emoji: ROLE_EMOJIS.IArbitre },
    ...setupState.gladiateurs.map((g) => ({
      id: g.id,
      name: g.name,
      role: "GladIAteur" as SpeakerRole,
      emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
    })),
    { id: "user", name: settingsState.settings.username, role: "user" as SpeakerRole, emoji: ROLE_EMOJIS.user },
  ];

  const usage: UsageLedger = arena.llmUsage
    ? { ...EMPTY_LEDGER, total: arena.llmUsage.total, calls: arena.llmUsage.calls, estimatedCostUsd: arena.llmUsage.estimatedCostUsd }
    : EMPTY_LEDGER;

  saveDiscussionHistory({
    id: arena.discussionId,
    topic: setupState.topic,
    discussionLanguage: setupState.discussionLanguage,
    modelName: describeActiveModel(settingsState.settings, [setupState.arbitre.model, ...setupState.gladiateurs.map((g) => g.model)]),
    participants,
    totalTurns: arena.currentTurn,
    synthesis: arena.synthesis,
    createdAt: new Date().toISOString(),
    messages: arena.messages,
    discussionMode: setupState.discussionMode,
    documentContent: arena.documentContent,
    documentFormat: arena.documentFormat,
    argumentMapMd: arena.argumentMapMarkdown,
    argumentMapMdBySpeaker: arena.argumentMapMarkdownBySpeaker,
    argumentMapJson: arena.argumentMap ? JSON.stringify(arena.argumentMap) : "",
    llmProvider: settingsState.settings.llmProvider,
    usage,
    estimatedCostUsd: arena.llmUsage?.estimatedCostUsd ?? 0,
    reportJson: serializeReport(buildReport(arena)),
    recaps: arena.recaps,
  }).catch((err) => console.error("Failed to save discussion history:", err));
}
