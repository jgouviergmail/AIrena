import { create } from "zustand";
import { logger } from "@/lib/logger";
import { saveDiscussionHistory } from "@/lib/tauri-api";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { describeActiveModel, useSettingsStore } from "@/stores/useSettingsStore";
import type {
  ArenaEvent,
  ArgumentMap,
  BanInfo,
  DirectiveData,
  DocumentFormat,
  EmotionalProfile,
  EmotionSnapshot,
  LlmUsage,
  Message,
  ParticipantInfo,
  ProviderKind,
  RagChunkInfo,
  RelationshipEdge,
  SpeakerRole,
  UsageLedger,
} from "@/lib/types";
import { EMPTY_LEDGER } from "@/lib/types";

/** Latest usage snapshot from the engine (mirrors the `llmUsageUpdated` event). */
export interface LlmUsageStatus {
  provider: ProviderKind;
  model: string;
  total: LlmUsage;
  calls: number;
  /** null: free provider or unknown price list */
  estimatedCostUsd: number | null;
  periodSpentUsd: number;
  budgetUsd: number;
  peak: boolean;
}

export interface BudgetAlertInfo {
  level: "warning" | "exceeded";
  spentUsd: number;
  budgetUsd: number;
}

/** Last emotional threshold crossed, for UI flashes (no event monkey-patching). */
export interface ThresholdCrossing {
  speakerId: string;
  axis: string;
  direction: string;
  value: number;
  /** Monotonic counter so identical crossings still trigger effects */
  seq: number;
}

/** Buffer key of a speaker's live reasoning stream (separate from their content stream). */
const REASONING_STREAM_SUFFIX = "::reasoning";
export const reasoningStreamKey = (speakerId: string) => `${speakerId}${REASONING_STREAM_SUFFIX}`;
/** Split a buffer key back into speaker id + stream kind. */
export function parseStreamKey(key: string): { speakerId: string; kind: "content" | "reasoning" } {
  return key.endsWith(REASONING_STREAM_SUFFIX)
    ? { speakerId: key.slice(0, -REASONING_STREAM_SUFFIX.length), kind: "reasoning" }
    : { speakerId: key, kind: "content" };
}

/** Callbacks from the token buffer in DiscussionFeed */
interface StreamBufferCallbacks {
  pushToken: (speakerId: string, token: string) => void;
  clearSpeaker: (speakerId: string) => void;
  clearAll: () => void;
}

/** Real-time engine activity for the status bar */
interface ActivityStatus {
  type: "thinking" | "writing" | "reacting" | "webSearch" | "wikiSearch" | "ragInjection" | "ragCacheHit" | "emotions" | "argumentMap" | "synthesis" | "determining";
  speakerName?: string;
}

interface ArenaState {
  discussionId: string | null;
  status: "idle" | "running" | "paused" | "synthesizing" | "ended";
  currentTurn: number;
  speakerOrder: string[];
  activeSpeakerId: string | null;
  activityStatus: ActivityStatus | null;
  messages: Message[];
  emotions: Map<string, EmotionalProfile>;
  emotionHistory: Map<string, EmotionSnapshot[]>;
  moodSummary: Map<string, string>;
  synthesis: string;
  synthesisStreaming: string;
  userTurnActive: boolean;
  interventionRequested: boolean;
  determiningOrder: boolean;
  webSearchCount: number;
  _pendingSearchCount: number;
  webSearchesPerMessage: Record<string, number>;
  wikiSearchCount: number;
  _pendingWikiCount: number;
  wikiSearchesPerMessage: Record<string, number>;
  wikiArticleUrlsPerMessage: Record<string, string[]>;
  _pendingWikiUrls: string[];
  ragChunkCount: number;
  _pendingRagCount: number;
  _pendingRagChunks: RagChunkInfo[];
  ragChunksPerMessage: Record<string, number>;
  ragChunkDetailsPerMessage: Record<string, RagChunkInfo[]>;
  documentContent: string;
  previousDocumentContent: string | null;
  documentFormat: DocumentFormat;
  documentLastEditor: string | null;
  argumentMapMarkdown: string;
  argumentMapMarkdownBySpeaker: string;
  argumentMapThesesCount: number;
  argumentMapArgumentsCount: number;
  /** Structured map (persisted with the discussion) */
  argumentMap: ArgumentMap | null;
  /** Node ids added by the latest extraction */
  argumentMapNewNodeIds: string[];
  argumentMapDroppedCount: number;
  directives: Map<string, DirectiveData>;
  bans: Map<string, BanInfo>;
  /** UserDriven: participants who passed this turn (in order) */
  passedSpeakerIds: string[];
  /** Cumulative reaction graph (relations tab) */
  relationships: RelationshipEdge[];
  llmUsage: LlmUsageStatus | null;
  budgetAlert: BudgetAlertInfo | null;
  lastThresholdCrossed: ThresholdCrossing | null;
  error: string | null;

  handleEvent: (event: ArenaEvent) => void;
  registerStreamBuffer: (cb: StreamBufferCallbacks) => void;
  unregisterStreamBuffer: () => void;
  reset: () => void;
}

const initialState = {
  discussionId: null as string | null,
  status: "idle" as const,
  currentTurn: 0,
  speakerOrder: [] as string[],
  activeSpeakerId: null as string | null,
  activityStatus: null as ActivityStatus | null,
  messages: [] as Message[],
  emotions: new Map<string, EmotionalProfile>(),
  emotionHistory: new Map<string, EmotionSnapshot[]>(),
  moodSummary: new Map<string, string>(),
  synthesis: "",
  synthesisStreaming: "",
  userTurnActive: false,
  interventionRequested: false,
  determiningOrder: false,
  webSearchCount: 0,
  _pendingSearchCount: 0,
  webSearchesPerMessage: {} as Record<string, number>,
  wikiSearchCount: 0,
  _pendingWikiCount: 0,
  wikiSearchesPerMessage: {} as Record<string, number>,
  wikiArticleUrlsPerMessage: {} as Record<string, string[]>,
  _pendingWikiUrls: [] as string[],
  ragChunkCount: 0,
  _pendingRagCount: 0,
  _pendingRagChunks: [] as RagChunkInfo[],
  ragChunksPerMessage: {} as Record<string, number>,
  ragChunkDetailsPerMessage: {} as Record<string, RagChunkInfo[]>,
  documentContent: "",
  previousDocumentContent: null as string | null,
  documentFormat: "none" as DocumentFormat,
  documentLastEditor: null as string | null,
  argumentMapMarkdown: "",
  argumentMapMarkdownBySpeaker: "",
  argumentMapThesesCount: 0,
  argumentMapArgumentsCount: 0,
  argumentMap: null as ArgumentMap | null,
  argumentMapNewNodeIds: [] as string[],
  argumentMapDroppedCount: 0,
  directives: new Map<string, DirectiveData>(),
  bans: new Map<string, BanInfo>(),
  passedSpeakerIds: [] as string[],
  relationships: [] as RelationshipEdge[],
  llmUsage: null as LlmUsageStatus | null,
  budgetAlert: null as BudgetAlertInfo | null,
  lastThresholdCrossed: null as ThresholdCrossing | null,
  error: null as string | null,
};

let _thresholdSeq = 0;

// Stream buffer ref — lives outside store to avoid triggering re-renders
let streamBuffer: StreamBufferCallbacks | null = null;

// Track first messageChunk per speaker to transition activity: "thinking" → "writing" (once)
let _writingSpeakerId: string | null = null;

// Synthesis buffer — same pattern: collect tokens, flush every 60ms
let synthBuffer: string[] = [];
let synthFlushTimer: ReturnType<typeof setInterval> | null = null;

function flushSynthBuffer() {
  if (synthBuffer.length === 0) return;
  const chunk = synthBuffer.join("");
  synthBuffer = [];
  useArenaStore.setState((s) => ({
    synthesisStreaming: s.synthesisStreaming + chunk,
  }));
}

function startSynthBuffering() {
  if (synthFlushTimer) return;
  synthFlushTimer = setInterval(flushSynthBuffer, 60);
}

function stopSynthBuffering() {
  if (synthFlushTimer) {
    clearInterval(synthFlushTimer);
    synthFlushTimer = null;
  }
  flushSynthBuffer();
  synthBuffer = [];
}

export const useArenaStore = create<ArenaState>((set) => ({
  ...initialState,

  registerStreamBuffer: (cb) => {
    streamBuffer = cb;
  },
  unregisterStreamBuffer: () => {
    streamBuffer = null;
  },

  handleEvent: (event: ArenaEvent) => {
    try {
    // Log all events except high-frequency streaming chunks
    if (event.type !== "messageChunk" && event.type !== "thoughtChunk" && event.type !== "synthesisChunk") {
      logger.info("arena", `Event: ${event.type}`, (event as Record<string, unknown>).data);
    }

    switch (event.type) {
      case "discussionStarted":
        logger.info("arena", `Discussion started: ${event.data.discussionId}`);
        set({
          discussionId: event.data.discussionId,
          status: "running",
          error: null,
        });
        break;

      case "messageChunk":
        // Delegate to the external token buffer (no Zustand state update)
        streamBuffer?.pushToken(event.data.speakerId, event.data.chunk);
        // Transition activity "thinking" → "writing" on first chunk per speaker
        if (_writingSpeakerId !== event.data.speakerId) {
          _writingSpeakerId = event.data.speakerId;
          set((s) => ({
            activityStatus: s.activityStatus
              ? { ...s.activityStatus, type: "writing" as const }
              : null,
          }));
        }
        break;

      case "messageComplete": {
        if (!event.data.message?.id) {
          console.error("Received messageComplete with invalid message", event.data);
          break;
        }
        // Clear the buffers (content + live reasoning) for this speaker
        streamBuffer?.clearSpeaker(event.data.message.speakerId);
        streamBuffer?.clearSpeaker(reasoningStreamKey(event.data.message.speakerId));
        set((s) => {
          const msg = {
            ...event.data.message,
            reactions: Array.isArray(event.data.message.reactions)
              ? event.data.message.reactions
              : [],
          };
          const wsPerMsg = s._pendingSearchCount > 0
            ? { ...s.webSearchesPerMessage, [msg.id]: s._pendingSearchCount }
            : s.webSearchesPerMessage;
          const wkPerMsg = s._pendingWikiCount > 0
            ? { ...s.wikiSearchesPerMessage, [msg.id]: s._pendingWikiCount }
            : s.wikiSearchesPerMessage;
          const wkUrls = s._pendingWikiUrls.length > 0
            ? { ...s.wikiArticleUrlsPerMessage, [msg.id]: [...s._pendingWikiUrls] }
            : s.wikiArticleUrlsPerMessage;
          const ragPerMsg = s._pendingRagCount > 0
            ? { ...s.ragChunksPerMessage, [msg.id]: s._pendingRagCount }
            : s.ragChunksPerMessage;
          const ragDetails = s._pendingRagChunks.length > 0
            ? { ...s.ragChunkDetailsPerMessage, [msg.id]: [...s._pendingRagChunks] }
            : s.ragChunkDetailsPerMessage;
          return {
            messages: [...s.messages, msg],
            webSearchesPerMessage: wsPerMsg,
            wikiSearchesPerMessage: wkPerMsg,
            wikiArticleUrlsPerMessage: wkUrls,
            ragChunksPerMessage: ragPerMsg,
            ragChunkDetailsPerMessage: ragDetails,
            _pendingSearchCount: 0,
            _pendingWikiCount: 0,
            _pendingWikiUrls: [],
            _pendingRagCount: 0,
            _pendingRagChunks: [],
          };
        });
        break;
      }

      case "reactionEmitted": {
        const targetId = event.data.messageId;
        const reaction = event.data.reaction;
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === targetId
              ? { ...m, reactions: [...(m.reactions ?? []), reaction] }
              : m,
          ),
          activityStatus: s.activityStatus?.type === "reacting"
            ? s.activityStatus
            : { type: "reacting" as const, speakerName: reaction.fromSpeakerName },
        }));
        break;
      }

      case "thoughtChunk":
        // Live reasoning (DeepSeek / Ollama think) — same 60 ms buffer, separate key
        streamBuffer?.pushToken(reasoningStreamKey(event.data.speakerId), event.data.chunk);
        break;

      case "thoughtComplete":
        // The thought is attached to the Message by the backend when MessageComplete arrives;
        // the live bubble stays visible until then.
        break;

      case "turnStarted":
        set((s) => {
          // Decrement ban counters
          const bans = new Map(s.bans);
          for (const [id, info] of bans) {
            if (info.remaining > 0) {
              if (info.justIssued) {
                bans.set(id, { ...info, justIssued: false });
              } else {
                bans.set(id, { ...info, remaining: info.remaining - 1 });
              }
            }
          }
          return {
            currentTurn: event.data.turnNumber,
            speakerOrder: event.data.speakerOrder,
            interventionRequested: false,
            determiningOrder: false,
            bans,
          };
        });
        break;

      case "turnSkipped":
        set({ determiningOrder: false });
        break;

      case "speakerPassed":
        set((s) => ({ passedSpeakerIds: [...s.passedSpeakerIds, event.data.speakerId] }));
        break;

      case "relationshipsUpdated":
        set({ relationships: event.data.edges });
        break;

      case "determiningOrder":
        set({ determiningOrder: true, activityStatus: { type: "determining" } });
        break;

      case "speakerActive": {
        // Clear streaming buffer for new speaker
        streamBuffer?.clearAll();
        _writingSpeakerId = null;
        const setup = useSetupStore.getState();
        const speakerName =
          setup.arbitre.id === event.data.speakerId
            ? setup.arbitre.name
            : setup.gladiateurs.find((g) => g.id === event.data.speakerId)?.name
              ?? event.data.speakerId;
        set({
          activeSpeakerId: event.data.speakerId,
          activityStatus: { type: "thinking", speakerName },
          _pendingSearchCount: 0,
          _pendingWikiCount: 0,
          _pendingWikiUrls: [],
          _pendingRagCount: 0,
          _pendingRagChunks: [],
        });
        break;
      }

      case "webSearchPerformed":
        set((s) => ({
          webSearchCount: s.webSearchCount + event.data.queries.length,
          _pendingSearchCount: s._pendingSearchCount + event.data.queries.length,
          activityStatus: { type: "webSearch" as const, speakerName: event.data.speakerName },
        }));
        break;

      case "wikiSearchPerformed":
        set((s) => ({
          wikiSearchCount: s.wikiSearchCount + event.data.queries.length,
          _pendingWikiCount: s._pendingWikiCount + event.data.queries.length,
          _pendingWikiUrls: [...s._pendingWikiUrls, ...event.data.articleUrls],
          activityStatus: { type: "wikiSearch" as const, speakerName: event.data.speakerName },
        }));
        break;

      case "ragContextInjected":
        set((s) => ({
          ragChunkCount: s.ragChunkCount + event.data.chunks.length,
          _pendingRagCount: s._pendingRagCount + event.data.chunks.length,
          _pendingRagChunks: [...s._pendingRagChunks, ...event.data.chunks],
          activityStatus: {
            type: event.data.cached ? "ragCacheHit" as const : "ragInjection" as const,
            speakerName: event.data.speakerName,
          },
        }));
        break;

      case "argumentMapUpdated":
        set({
          argumentMapMarkdown: event.data.markdown,
          argumentMapMarkdownBySpeaker: event.data.markdownBySpeaker,
          argumentMapThesesCount: event.data.thesesCount,
          argumentMapArgumentsCount: event.data.argumentsCount,
          argumentMap: event.data.map,
          argumentMapNewNodeIds: event.data.newNodeIds,
          argumentMapDroppedCount: event.data.droppedCount,
          activityStatus: { type: "argumentMap" },
        });
        break;

      case "llmUsageUpdated":
        set({ llmUsage: event.data });
        break;

      case "budgetAlert":
        set({ budgetAlert: event.data });
        break;

      case "emotionUpdated":
        set((s) => {
          const em = new Map(s.emotions);
          em.set(event.data.speakerId, event.data.emotions);
          const ms = event.data.moodSummary
            ? new Map(s.moodSummary).set(event.data.speakerId, event.data.moodSummary)
            : s.moodSummary;
          return {
            emotions: em,
            moodSummary: ms,
            activityStatus: s.activityStatus?.type === "emotions"
              ? s.activityStatus
              : { type: "emotions" as const },
          };
        });
        break;

      case "emotionHistoryUpdate":
        set((s) => {
          const eh = new Map(s.emotionHistory);
          eh.set(event.data.speakerId, event.data.history);
          return { emotionHistory: eh };
        });
        break;

      case "emotionalThresholdCrossed":
        set({ lastThresholdCrossed: { ...event.data, seq: ++_thresholdSeq } });
        break;

      case "documentUpdated":
        set((s) => ({
          previousDocumentContent: s.documentContent || null,
          documentContent: event.data.content,
          documentFormat: event.data.format as DocumentFormat,
          documentLastEditor: event.data.speakerName,
        }));
        break;

      case "directiveGenerated":
        set((s) => {
          const d = new Map(s.directives);
          d.set(event.data.speakerId, {
            speechAct: event.data.speechAct,
            emotionBehavior: event.data.emotionBehavior ?? null,
            relationshipSummary: event.data.relationshipSummary,
            focusSpeaker: event.data.focusSpeaker ?? null,
            reasoningLevel: event.data.reasoningLevel ?? "off",
          });
          return { directives: d };
        });
        break;

      case "banIssued": {
        set((s) => {
          const bans = new Map(s.bans);
          const prev = bans.get(event.data.bannedId);
          bans.set(event.data.bannedId, {
            remaining: event.data.duration,
            totalBans: (prev?.totalBans ?? 0) + 1,
            justIssued: true,
          });
          return { bans };
        });
        break;
      }

      case "banLifted": {
        set((s) => {
          const bans = new Map(s.bans);
          const prev = bans.get(event.data.speakerId);
          if (prev) {
            bans.set(event.data.speakerId, { remaining: 0, totalBans: prev.totalBans, justIssued: false });
          }
          return { bans };
        });
        break;
      }

      case "userTurnReady":
        // A new user turn starts a new round of respond/pass decisions
        set({ userTurnActive: true, interventionRequested: false, passedSpeakerIds: [] });
        break;

      case "userTurnTimeout":
        set({ userTurnActive: false });
        break;

      case "pauseConfirmed":
        set({ status: "paused", activityStatus: null });
        break;

      case "resumeConfirmed":
        set({ status: "running" });
        break;

      case "synthesisChunk":
        // Buffer synthesis tokens (same pattern as message streaming)
        synthBuffer.push(event.data.chunk);
        if (!synthFlushTimer) {
          startSynthBuffering();
          set({ status: "synthesizing", activityStatus: { type: "synthesis" } });
        }
        break;

      case "synthesisComplete":
        stopSynthBuffering();
        set({ synthesis: event.data.summary, synthesisStreaming: "", status: "ended", activityStatus: null });
        break;

      case "discussionEnded": {
        stopSynthBuffering();

        // Auto-save discussion to history before setting status
        const arenaState = useArenaStore.getState();
        if (arenaState.discussionId && arenaState.messages.length > 0) {
          const setupState = useSetupStore.getState();
          const settingsState = useSettingsStore.getState();

          const participants: ParticipantInfo[] = [
            {
              id: setupState.arbitre.id,
              name: setupState.arbitre.name,
              role: "IArbitre" as SpeakerRole,
              emoji: ROLE_EMOJIS.IArbitre,
            },
            ...setupState.gladiateurs.map((g) => ({
              id: g.id,
              name: g.name,
              role: "GladIAteur" as SpeakerRole,
              emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
            })),
            {
              id: "user",
              name: settingsState.settings.username,
              role: "user" as SpeakerRole,
              emoji: ROLE_EMOJIS.user,
            },
          ];

          const usage: UsageLedger = arenaState.llmUsage
            ? {
                ...EMPTY_LEDGER,
                total: arenaState.llmUsage.total,
                calls: arenaState.llmUsage.calls,
                estimatedCostUsd: arenaState.llmUsage.estimatedCostUsd,
              }
            : EMPTY_LEDGER;
          saveDiscussionHistory({
            id: arenaState.discussionId,
            topic: setupState.topic,
            discussionLanguage: setupState.discussionLanguage,
            modelName: describeActiveModel(settingsState.settings),
            participants,
            totalTurns: arenaState.currentTurn,
            synthesis: arenaState.synthesis,
            createdAt: new Date().toISOString(),
            messages: arenaState.messages,
            discussionMode: setupState.discussionMode,
            documentContent: arenaState.documentContent,
            documentFormat: arenaState.documentFormat,
            argumentMapMd: arenaState.argumentMapMarkdown,
            argumentMapMdBySpeaker: arenaState.argumentMapMarkdownBySpeaker,
            argumentMapJson: arenaState.argumentMap ? JSON.stringify(arenaState.argumentMap) : "",
            llmProvider: settingsState.settings.llmProvider,
            usage,
            estimatedCostUsd: arenaState.llmUsage?.estimatedCostUsd ?? 0,
          }).catch((err) =>
            console.error("Failed to save discussion history:", err),
          );
        }

        // The engine updated server-owned counters (cloud spend period, Tavily
        // credits): refresh the settings store so a later save never writes
        // back the values hydrated before the discussion.
        useSettingsStore.getState().hydrate().catch(() => {});

        set({ status: "ended", determiningOrder: false, activityStatus: null });
        break;
      }

      case "error":
        logger.error("arena", `Engine error: ${event.data.message}`);
        set({ error: event.data.message, determiningOrder: false });
        break;
    }
    } catch (err) {
      const msg = err instanceof Error ? `${err.message}\n${err.stack}` : String(err);
      logger.error("arena", `handleEvent crash on "${event.type}": ${msg}`);
      console.error("handleEvent crash:", event, err);
    }
  },

  reset: () => {
    streamBuffer?.clearAll();
    stopSynthBuffering();
    _writingSpeakerId = null;
    set({
      ...initialState,
      emotions: new Map<string, EmotionalProfile>(),
      emotionHistory: new Map<string, EmotionSnapshot[]>(),
      moodSummary: new Map<string, string>(),
      documentContent: "",
      previousDocumentContent: null,
      documentFormat: "none" as DocumentFormat,
      documentLastEditor: null,
      argumentMapMarkdown: "",
      argumentMapMarkdownBySpeaker: "",
      argumentMapThesesCount: 0,
      argumentMapArgumentsCount: 0,
      argumentMap: null,
      argumentMapNewNodeIds: [],
      argumentMapDroppedCount: 0,
      directives: new Map<string, DirectiveData>(),
      bans: new Map<string, BanInfo>(),
      passedSpeakerIds: [],
      relationships: [],
      llmUsage: null,
      budgetAlert: null,
      lastThresholdCrossed: null,
    });
  },
}));
