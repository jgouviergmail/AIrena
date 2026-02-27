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
  temperature: 0.8,
  topP: 0.9,
  topK: 40,
  numPredict: 2048,
  numCtx: 8192,
  repeatPenalty: 1.3,
};

export interface GladIAteurConfig {
  id: string;
  name: string;
  interventionNumber: number;
  systemPrompt: string;
  llmParams: LlmParams;
  emoji?: string;
  sourceProfileId?: string;
  initialEmotions?: string;
}

export interface IArbitreConfig {
  id: string;
  name: string;
  systemPrompt: string;
  turnDistribution: "sequential" | "random" | "democratic" | "authoritarian";
  llmParams: LlmParams;
  webSearchIntro?: boolean;
  wikiSearchIntro?: boolean;
}

export interface DiscussionConfig {
  topic: string;
  discussionLanguage: string;
  arbitre: IArbitreConfig;
  gladiateurs: GladIAteurConfig[];
  maxTurns: number | null;
  userName: string;
  userInterventionTimeoutSecs: number;
  webSearchPool: number;
  wikiSearchPool: number;
  discussionMode: DiscussionMode;
  documentFormat: DocumentFormat;
  argumentMapEnabled: boolean;
  documentInjectionMode: DocumentInjectionMode;
}

export type DiscussionMode = "debate" | "ideation" | "coConstruction" | "userDriven" | "socratic" | "tutorial" | "critiqueReview" | "collaborativeFiction";
export type DocumentFormat = "none" | "txt" | "md" | "csv";
export type DocumentInjectionMode = "rag" | "fullInjection";

export type SpeakerRole = "IArbitre" | "GladIAteur" | "user";
export type ReactionType = "like" | "dislike";

export interface Reaction {
  fromSpeakerId: string;
  fromSpeakerName: string;
  reactionType: ReactionType;
  targetMessageId: string;
  justification?: string;
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

export interface EmotionSnapshot {
  turn: number;
  emotions: EmotionalProfile;
}

export interface AppSettings {
  username: string;
  language: string;
  theme: string;
  ollamaUrl: string;
  ollamaModel: string;
  emotionDriven: boolean;
  tavilyApiKey: string;
  tavilyPeriodStart: string;
  tavilyUsageCount: number;
  tavilyUsageHistory: string;
  embeddingModel: string;
  licenseKey: string;
  tokenBudgetPriorities: string;
  numCtx: number;
}

export interface LicenseStatus {
  valid: boolean;
  email: string;
  expiresAt: number;
  maxDiscussions: number;
  discussionsUsed: number;
  error: string | null;
}

export interface TavilyPeriodHistory {
  periodStart: string;
  periodEnd: string;
  usageCount: number;
}

export interface PredefinedProfile {
  id: string;
  name: string;
  personality: string;
  systemPrompt: string;
  isBuiltin: boolean;
  profileType: string;
  category: string;
  initialEmotions?: string;
}

export interface ModelInfo {
  name: string;
  size: number;
  digest: string;
}

// -- Discussion history types --

export interface ParticipantInfo {
  id: string;
  name: string;
  role: SpeakerRole;
  emoji: string;
}

export interface SaveDiscussionRequest {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  synthesis: string;
  createdAt: string;
  messages: Message[];
  discussionMode: string;
  documentContent: string;
  documentFormat: string;
  argumentMapMd: string;
  argumentMapMdBySpeaker: string;
}

export interface DiscussionSummary {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  hasSynthesis: boolean;
  createdAt: string;
  discussionMode: string;
  documentFormat: string;
  hasArgumentMap: boolean;
}

export interface DiscussionDetail {
  id: string;
  topic: string;
  discussionLanguage: string;
  modelName: string;
  participants: ParticipantInfo[];
  totalTurns: number;
  synthesis: string;
  createdAt: string;
  messages: Message[];
  discussionMode: string;
  documentContent: string;
  documentFormat: string;
  argumentMapMd: string;
  argumentMapMdBySpeaker: string;
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
  | { type: "determiningOrder"; data: { turnNumber: number } }
  | { type: "speakerActive"; data: { speakerId: string } }
  | {
      type: "emotionUpdated";
      data: { speakerId: string; emotions: EmotionalProfile; moodSummary?: string };
    }
  | {
      type: "emotionHistoryUpdate";
      data: { speakerId: string; history: EmotionSnapshot[] };
    }
  | {
      type: "emotionalThresholdCrossed";
      data: { speakerId: string; axis: string; direction: string; value: number };
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
  | {
      type: "webSearchPerformed";
      data: {
        speakerId: string;
        speakerName: string;
        queries: string[];
        resultsCount: number;
        poolUsed: number;
      };
    }
  | {
      type: "wikiSearchPerformed";
      data: {
        speakerId: string;
        speakerName: string;
        queries: string[];
        resultsCount: number;
        poolUsed: number;
        articleUrls: string[];
      };
    }
  | {
      type: "documentUpdated";
      data: {
        speakerId: string;
        speakerName: string;
        content: string;
        format: string;
      };
    }
  | {
      type: "directiveGenerated";
      data: {
        speakerId: string;
        speakerName: string;
        speechAct: string;
        emotionBehavior: string | null;
        relationshipSummary: string;
      };
    }
  | {
      type: "ragContextInjected";
      data: {
        speakerId: string;
        speakerName: string;
        chunks: RagChunkInfo[];
        cached?: boolean;
      };
    }
  | {
      type: "argumentMapUpdated";
      data: {
        markdown: string;
        markdownBySpeaker: string;
        thesesCount: number;
        argumentsCount: number;
      };
    }
  | { type: "discussionEnded"; data: null }
  | { type: "error"; data: { message: string } };

// RAG types
export interface RagDocumentInfo {
  docId: string;
  fileName: string;
  format: string;
  chunkCount: number;
  charCount: number;
}

export interface RagChunkInfo {
  fileName: string;
  chunkIndex: number;
  preview: string;
  relevanceScore: number;
}

// Directive data for UI visualization
export interface DirectiveData {
  speechAct: string;
  emotionBehavior: string | null;
  relationshipSummary: string;
}

// Ban tracking for emotion sidebar
export interface BanInfo {
  remaining: number;
  totalBans: number;
  justIssued: boolean;
}

// ── Token Budget types ──────────────────────────────────────────────────

export interface ModelArchInfo {
  family: string;
  blockCount: number;
  headCount: number;
  headCountKv: number;
  embeddingLength: number;
  headDim: number;
  contextLength: number;
  quantization: string;
  kvBytesPerToken: number;
}

export interface VramStatus {
  gpuName: string;
  totalMb: number;
  freeMb: number;
  detectionMethod: string;
}

export interface ModelBudgetInfo {
  arch: ModelArchInfo | null;
  vram: VramStatus | null;
  recommendedNumCtx: number | null;
  currentNumCtx: number | null;
  ollamaVramMb: number | null;
  supportsThink: boolean;
  warnings: string[];
}

export type BudgetSection =
  | "currentTurnMessages"
  | "immediateMemory"
  | "contextualSummary"
  | "cognitiveDirectives"
  | "arbitreDirectives"
  | "fullDocument"
  | "ragContext"
  | "webWikiSearch"
  | "positionalMap";

/** Sections whose priority rank can be configured by the user in Settings. */
export const CONFIGURABLE_BUDGET_SECTIONS: BudgetSection[] = [
  "currentTurnMessages",
  "immediateMemory",
  "contextualSummary",
  "cognitiveDirectives",
  "arbitreDirectives",
  "webWikiSearch",
  "positionalMap",
];

export interface SectionPriority {
  section: BudgetSection;
  rank: number;
  floor: number;
  ceiling: number;
}

export interface BudgetFeatures {
  webSearchEnabled: boolean;
  wikiSearchEnabled: boolean;
  ragEnabled: boolean;
  documentChars: number;
}

export interface BudgetParams {
  numCtx: number;
  numPredict: number;
  systemPromptChars: number;
  nGladiateurs: number;
  language: string;
  features: BudgetFeatures;
}

export interface SectionAllocation {
  section: BudgetSection;
  allocatedChars: number;
  floorChars: number;
  ceilingChars: number;
}

export type BudgetQualityLevel = "optimal" | "degraded" | "insufficient";

export interface TokenBudgetPreview {
  totalTokens: number;
  reservedTokens: number;
  availableTokens: number;
  sections: SectionAllocation[];
  warnings: string[];
  fullDocumentMode: boolean;
  charsPerToken: number;
  qualityLevel: BudgetQualityLevel;
  documentAvailableTokens: number;
}
