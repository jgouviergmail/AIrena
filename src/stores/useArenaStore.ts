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

interface ArenaState {
  discussionId: string | null;
  status: "idle" | "running" | "paused" | "synthesizing" | "ended";
  currentTurn: number;
  speakerOrder: string[];
  activeSpeakerId: string | null;
  messages: Message[];
  streamingContent: Map<string, string>;
  streamingThoughts: Map<string, string>;
  emotions: Map<string, EmotionalProfile>;
  synthesis: string;
  synthesisStreaming: string;
  userTurnActive: boolean;
  error: string | null;

  handleEvent: (event: ArenaEvent) => void;
  pushStreamToken: (speakerId: string, chunk: string) => void;
  pushThoughtToken: (speakerId: string, chunk: string) => void;
  getStreamingContent: (speakerId: string) => string;
  getStreamingThought: (speakerId: string) => string;
  reset: () => void;
}

const initialState = {
  discussionId: null as string | null,
  status: "idle" as const,
  currentTurn: 0,
  speakerOrder: [] as string[],
  activeSpeakerId: null as string | null,
  messages: [] as Message[],
  streamingContent: new Map<string, string>(),
  streamingThoughts: new Map<string, string>(),
  emotions: new Map<string, EmotionalProfile>(),
  synthesis: "",
  synthesisStreaming: "",
  userTurnActive: false,
  error: null as string | null,
};

export const useArenaStore = create<ArenaState>((set, get) => ({
  ...initialState,

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
        get().pushStreamToken(event.data.speakerId, event.data.chunk);
        break;

      case "messageComplete": {
        if (!event.data.message?.id) {
          console.error("Received messageComplete with invalid message", event.data);
          break;
        }
        set((s) => {
          // Defensive copy: ensure reactions is always an array (serialization edge cases)
          const msg = {
            ...event.data.message,
            reactions: Array.isArray(event.data.message.reactions)
              ? event.data.message.reactions
              : [],
          };
          const sc = new Map(s.streamingContent);
          sc.delete(msg.speakerId);
          return {
            messages: [...s.messages, msg],
            streamingContent: sc,
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
        get().pushThoughtToken(event.data.speakerId, event.data.chunk);
        break;

      case "thoughtComplete":
        // Just clear the streaming thought — the thought is attached to the
        // Message by the backend when MessageComplete arrives
        set((s) => {
          const st = new Map(s.streamingThoughts);
          st.delete(event.data.speakerId);
          return { streamingThoughts: st };
        });
        break;

      case "turnStarted":
        set({
          currentTurn: event.data.turnNumber,
          speakerOrder: event.data.speakerOrder,
        });
        break;

      case "turnSkipped":
        // Just a notification, no state change needed
        break;

      case "speakerActive":
        // Clear any lingering streaming content from previous speakers
        set(() => ({
          activeSpeakerId: event.data.speakerId,
          streamingContent: new Map<string, string>(),
          streamingThoughts: new Map<string, string>(),
        }));
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
        set((s) => ({
          status: "synthesizing",
          synthesisStreaming: s.synthesisStreaming + event.data.chunk,
        }));
        break;

      case "synthesisComplete":
        set({ synthesis: event.data.summary, synthesisStreaming: "", status: "ended" });
        break;

      case "discussionEnded":
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

  pushStreamToken: (speakerId, chunk) => {
    set((s) => {
      const sc = new Map(s.streamingContent);
      sc.set(speakerId, (sc.get(speakerId) ?? "") + chunk);
      return { streamingContent: sc };
    });
  },

  pushThoughtToken: (speakerId, chunk) => {
    set((s) => {
      const st = new Map(s.streamingThoughts);
      st.set(speakerId, (st.get(speakerId) ?? "") + chunk);
      return { streamingThoughts: st };
    });
  },

  getStreamingContent: (speakerId) => {
    return get().streamingContent.get(speakerId) ?? "";
  },

  getStreamingThought: (speakerId) => {
    return get().streamingThoughts.get(speakerId) ?? "";
  },

  reset: () =>
    set({
      ...initialState,
      streamingContent: new Map<string, string>(),
      streamingThoughts: new Map<string, string>(),
      emotions: new Map<string, EmotionalProfile>(),
    }),
}));
