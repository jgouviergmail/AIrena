// Mirror of Rust types — keep in sync with src-tauri/src/models/

export interface LlmParams {
  temperature: number;
  topP: number;
  topK: number;
  numPredict: number;
  numCtx: number;
  repeatPenalty: number;
}

export const DEFAULT_LLM_PARAMS: LlmParams = {
  temperature: 0.7,
  topP: 0.9,
  topK: 40,
  numPredict: 1024,
  numCtx: 8192,
  repeatPenalty: 1.1,
};

export interface GladIAteurConfig {
  id: string;
  name: string;
  interventionNumber: number;
  systemPrompt: string;
  llmParams: LlmParams;
}

export interface IArbitreConfig {
  id: string;
  name: string;
  systemPrompt: string;
  turnDistribution: "sequential" | "random";
  llmParams: LlmParams;
}

export interface DiscussionConfig {
  topic: string;
  discussionLanguage: string;
  arbitre: IArbitreConfig;
  gladiateurs: GladIAteurConfig[];
  maxTurns: number | null;
  userName: string;
  userInterventionTimeoutSecs: number;
}

export type SpeakerRole = "IArbitre" | "GladIAteur" | "user";
export type ReactionType = "like" | "dislike";

export interface Reaction {
  fromSpeakerId: string;
  fromSpeakerName: string;
  reactionType: ReactionType;
  targetMessageId: string;
}

export interface Message {
  id: string;
  discussionId: string;
  turnNumber: number;
  speakerId: string;
  speakerName: string;
  role: SpeakerRole;
  content: string;
  innerThought: string | null;
  reactions: Reaction[];
  isBanNotification: boolean;
  timestamp: string;
}

export interface EmotionalProfile {
  engagement: number;
  accord: number;
  confiance: number;
  frustration: number;
  curiosite: number;
  enthousiasme: number;
}

export interface AppSettings {
  username: string;
  language: string;
  theme: string;
  ollamaUrl: string;
  ollamaModel: string;
  emotionDriven: boolean;
}

export interface PredefinedProfile {
  id: string;
  name: string;
  personality: string;
  systemPrompt: string;
  isBuiltin: boolean;
}

export interface ModelInfo {
  name: string;
  size: number;
  digest: string;
}

// ArenaEvent — tagged union (discriminated via "type" field)
export type ArenaEvent =
  | { type: "discussionStarted"; data: { discussionId: string } }
  | { type: "messageChunk"; data: { speakerId: string; chunk: string } }
  | { type: "messageComplete"; data: { message: Message } }
  | {
      type: "reactionEmitted";
      data: { messageId: string; reaction: Reaction };
    }
  | { type: "thoughtChunk"; data: { speakerId: string; chunk: string } }
  | { type: "thoughtComplete"; data: { speakerId: string; thought: string } }
  | {
      type: "turnStarted";
      data: { turnNumber: number; speakerOrder: string[] };
    }
  | {
      type: "turnSkipped";
      data: { reason: string; nextAvailableTurn: number };
    }
  | { type: "speakerActive"; data: { speakerId: string } }
  | {
      type: "emotionUpdated";
      data: { speakerId: string; emotions: EmotionalProfile };
    }
  | {
      type: "banIssued";
      data: {
        bannedId: string;
        bannedName: string;
        reason: string;
        duration: number;
      };
    }
  | {
      type: "banLifted";
      data: { speakerId: string; speakerName: string };
    }
  | { type: "userTurnReady"; data: null }
  | { type: "userTurnTimeout"; data: null }
  | { type: "pauseConfirmed"; data: null }
  | { type: "resumeConfirmed"; data: null }
  | { type: "synthesisChunk"; data: { chunk: string } }
  | { type: "synthesisComplete"; data: { summary: string } }
  | { type: "discussionEnded"; data: null }
  | { type: "error"; data: { message: string } };
