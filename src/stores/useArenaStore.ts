import { create } from "zustand";
import { logger } from "@/lib/logger";
import { saveDiscussionHistory } from "@/lib/tauri-api";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import type {
  ArenaEvent,
  BanInfo,
  DirectiveData,
  DocumentFormat,
  EmotionalProfile,
  EmotionSnapshot,
  Message,
  ParticipantInfo,
  SpeakerRole,
} from "@/lib/types";

/** Callbacks from the token buffer in DiscussionFeed */
interface StreamBufferCallbacks {
  pushToken: (speakerId: string, token: string) => void;
  clearSpeaker: (speakerId: string) => void;
  clearAll: () => void;
}

interface ArenaState {
  discussionId: string | null;
  status: "idle" | "running" | "paused" | "synthesizing" | "ended";
  currentTurn: number;
  speakerOrder: string[];
  activeSpeakerId: string | null;
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
  documentContent: string;
  previousDocumentContent: string | null;
  documentFormat: DocumentFormat;
  documentLastEditor: string | null;
  directives: Map<string, DirectiveData>;
  bans: Map<string, BanInfo>;
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
  documentContent: "",
  previousDocumentContent: null as string | null,
  documentFormat: "none" as DocumentFormat,
  documentLastEditor: null as string | null,
  directives: new Map<string, DirectiveData>(),
  bans: new Map<string, BanInfo>(),
  error: null as string | null,
};

// Stream buffer ref — lives outside store to avoid triggering re-renders
let streamBuffer: StreamBufferCallbacks | null = null;

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
        break;

      case "messageComplete": {
        if (!event.data.message?.id) {
          console.error("Received messageComplete with invalid message", event.data);
          break;
        }
        // Clear the buffer for this speaker
        streamBuffer?.clearSpeaker(event.data.message.speakerId);
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
          return {
            messages: [...s.messages, msg],
            webSearchesPerMessage: wsPerMsg,
            wikiSearchesPerMessage: wkPerMsg,
            wikiArticleUrlsPerMessage: wkUrls,
            _pendingSearchCount: 0,
            _pendingWikiCount: 0,
            _pendingWikiUrls: [],
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
        }));
        break;
      }

      case "thoughtChunk":
        // Thoughts also go through the buffer (reuse same mechanism)
        // Not rendered separately for now — just ignore to avoid perf cost
        break;

      case "thoughtComplete":
        // The thought is attached to the Message by the backend when MessageComplete arrives
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

      case "determiningOrder":
        set({ determiningOrder: true });
        break;

      case "speakerActive":
        // Clear streaming buffer for new speaker
        streamBuffer?.clearAll();
        set({
          activeSpeakerId: event.data.speakerId,
          _pendingSearchCount: 0,
          _pendingWikiCount: 0,
          _pendingWikiUrls: [],
        });
        break;

      case "webSearchPerformed":
        set((s) => ({
          webSearchCount: s.webSearchCount + event.data.queries.length,
          _pendingSearchCount: s._pendingSearchCount + event.data.queries.length,
        }));
        break;

      case "wikiSearchPerformed":
        set((s) => ({
          wikiSearchCount: s.wikiSearchCount + event.data.queries.length,
          _pendingWikiCount: s._pendingWikiCount + event.data.queries.length,
          _pendingWikiUrls: [...s._pendingWikiUrls, ...event.data.articleUrls],
        }));
        break;

      case "emotionUpdated":
        set((s) => {
          const em = new Map(s.emotions);
          em.set(event.data.speakerId, event.data.emotions);
          const ms = event.data.moodSummary
            ? new Map(s.moodSummary).set(event.data.speakerId, event.data.moodSummary)
            : s.moodSummary;
          return { emotions: em, moodSummary: ms };
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
        // No state update — handled by EmotionSidebar via CSS animations
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
        set({ userTurnActive: true, interventionRequested: false });
        break;

      case "userTurnTimeout":
        set({ userTurnActive: false });
        break;

      case "pauseConfirmed":
        set({ status: "paused" });
        break;

      case "resumeConfirmed":
        set({ status: "running" });
        break;

      case "synthesisChunk":
        // Buffer synthesis tokens (same pattern as message streaming)
        synthBuffer.push(event.data.chunk);
        if (!synthFlushTimer) {
          startSynthBuffering();
          set({ status: "synthesizing" });
        }
        break;

      case "synthesisComplete":
        stopSynthBuffering();
        set({ synthesis: event.data.summary, synthesisStreaming: "", status: "ended" });
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

          saveDiscussionHistory({
            id: arenaState.discussionId,
            topic: setupState.topic,
            discussionLanguage: setupState.discussionLanguage,
            modelName: settingsState.settings.ollamaModel,
            participants,
            totalTurns: arenaState.currentTurn,
            synthesis: arenaState.synthesis,
            createdAt: new Date().toISOString(),
            messages: arenaState.messages,
            discussionMode: setupState.discussionMode,
            documentContent: arenaState.documentContent,
            documentFormat: arenaState.documentFormat,
          }).catch((err) =>
            console.error("Failed to save discussion history:", err),
          );
        }

        set({ status: "ended", determiningOrder: false });
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
    set({
      ...initialState,
      emotions: new Map<string, EmotionalProfile>(),
      emotionHistory: new Map<string, EmotionSnapshot[]>(),
      moodSummary: new Map<string, string>(),
      documentContent: "",
      previousDocumentContent: null,
      documentFormat: "none" as DocumentFormat,
      documentLastEditor: null,
      directives: new Map<string, DirectiveData>(),
      bans: new Map<string, BanInfo>(),
    });
  },
}));
