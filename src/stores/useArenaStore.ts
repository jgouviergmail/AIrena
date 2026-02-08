import { create } from "zustand";
import i18n from "@/i18n/config";
import { logger } from "@/lib/logger";
import type {
  ArenaEvent,
  EmotionalProfile,
  Message,
  SpeakerRole,
} from "@/lib/types";

/** Build a system notification message (ban/unban) for the feed. */
function makeSystemMessage(
  discussionId: string,
  turnNumber: number,
  content: string,
  idPrefix: string,
): Message {
  return {
    id: `${idPrefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    discussionId,
    turnNumber,
    speakerId: "system",
    speakerName: "IArbitre",
    role: "IArbitre" as SpeakerRole,
    content,
    innerThought: null,
    reactions: [],
    isBanNotification: true,
    timestamp: new Date().toISOString(),
  };
}

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
  synthesis: string;
  synthesisStreaming: string;
  userTurnActive: boolean;
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
  synthesis: "",
  synthesisStreaming: "",
  userTurnActive: false,
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
          return {
            messages: [...s.messages, msg],
          };
        });
        break;
      }

      case "reactionEmitted":
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === event.data.messageId
              ? { ...m, reactions: [...(m.reactions ?? []), event.data.reaction] }
              : m,
          ),
        }));
        break;

      case "thoughtChunk":
        // Thoughts also go through the buffer (reuse same mechanism)
        // Not rendered separately for now — just ignore to avoid perf cost
        break;

      case "thoughtComplete":
        // The thought is attached to the Message by the backend when MessageComplete arrives
        break;

      case "turnStarted":
        set({
          currentTurn: event.data.turnNumber,
          speakerOrder: event.data.speakerOrder,
        });
        break;

      case "turnSkipped":
        break;

      case "speakerActive":
        // Clear streaming buffer for new speaker
        streamBuffer?.clearAll();
        set({
          activeSpeakerId: event.data.speakerId,
        });
        break;

      case "emotionUpdated":
        set((s) => {
          const em = new Map(s.emotions);
          em.set(event.data.speakerId, event.data.emotions);
          return { emotions: em };
        });
        break;

      case "banIssued":
        set((s) => ({
          messages: [
            ...s.messages,
            makeSystemMessage(
              s.discussionId ?? "",
              s.currentTurn,
              i18n.t("arena.banned", { name: event.data.bannedName, duration: event.data.duration, reason: event.data.reason }),
              "ban",
            ),
          ],
        }));
        break;

      case "banLifted":
        set((s) => ({
          messages: [
            ...s.messages,
            makeSystemMessage(
              s.discussionId ?? "",
              s.currentTurn,
              i18n.t("arena.banLifted", { name: event.data.speakerName }),
              "unban",
            ),
          ],
        }));
        break;

      case "userTurnReady":
        set({ userTurnActive: true });
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

      case "discussionEnded":
        stopSynthBuffering();
        set({ status: "ended" });
        break;

      case "error":
        logger.error("arena", `Engine error: ${event.data.message}`);
        set({ error: event.data.message });
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
    });
  },
}));
