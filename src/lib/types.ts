// Mirror of Rust types — keep in sync with src-tauri/src/models/

// ── LLM provider ────────────────────────────────────────────────────────

/** Backend serving the discussion (settings.llmProvider). */
export type ProviderKind = "ollama" | "deepseek";

/** Reasoning ("thinking") intensity. "auto" is resolved per call by the engine. */
export type ReasoningLevel = "off" | "low" | "high" | "max" | "auto";

export const REASONING_LEVELS: ReasoningLevel[] = ["auto", "off", "low", "high", "max"];

export interface LlmParams {
  temperature: number;
  topP: number;
  topK: number;
  numPredict: number;
  numCtx: number;
  repeatPenalty: number;
  /** Per-speaker override; undefined inherits the global setting. */
  reasoningLevel?: ReasoningLevel;
}

export type CallKind =
  | "introduction"
  | "thought"
  | "intervention"
  | "reaction"
  | "moderation"
  | "memory"
  | "emotion"
  | "vote"
  | "searchDecision"
  | "ragSelect"
  | "documentUpdate"
  | "argumentMap"
  | "synthesis"
  | "socratic"
  | "respondOrPass";

/** Token usage reported by a provider (all zero when unknown). */
export interface LlmUsage {
  promptTokens: number;
  cachedTokens: number;
  completionTokens: number;
  reasoningTokens: number;
}

export const EMPTY_USAGE: LlmUsage = { promptTokens: 0, cachedTokens: 0, completionTokens: 0, reasoningTokens: 0 };

/** Accumulated usage for a discussion. */
export interface UsageLedger {
  total: LlmUsage;
  calls: number;
  byCallKind: Partial<Record<CallKind, LlmUsage>>;
  bySpeaker: Record<string, LlmUsage>;
  /** null: free provider or unknown price list */
  estimatedCostUsd: number | null;
}

export const EMPTY_LEDGER: UsageLedger = { total: EMPTY_USAGE, calls: 0, byCallKind: {}, bySpeaker: {}, estimatedCostUsd: null };

export interface PeriodUsage {
  usage: LlmUsage;
  costUsd: number;
  discussions: number;
}

export interface PeriodHistoryEntry extends PeriodUsage {
  periodStart: string;
  periodEnd: string;
}

/** Current rolling period of cloud spend (Settings gauge + pre-flight). */
export interface LlmUsagePeriod {
  provider: ProviderKind;
  model: string;
  periodStart: string;
  periodEnd: string;
  usage: PeriodUsage;
  /** Monthly cap in USD (0 = unlimited) */
  budgetUsd: number;
  history: PeriodHistoryEntry[];
  pricingDate: string;
  peakNow: boolean;
}

/** Peak-hour price list of one model (USD per million tokens). */
export interface ModelPriceInfo {
  model: string;
  inputCacheHit: number;
  inputCacheMiss: number;
  output: number;
}

/** Provider limits and defaults owned by the backend. */
export interface LlmConstants {
  deepseekDefaultModel: string;
  deepseekKnownModels: string[];
  deepseekDefaultContextBudget: number;
  deepseekMinContextBudget: number;
  deepseekMaxContextBudget: number;
  deepseekMaxNumPredictUi: number;
  deepseekPricingDate: string;
  deepseekPricing: ModelPriceInfo[];
  deepseekOffpeakFactor: number;
  /** UTC hour windows [start, end) of peak pricing, Monday–Friday */
  deepseekPeakWindowsUtc: [number, number][];
  deepseekTopPMinThinking: number;
  budgetWarnRatio: number;
}

export interface DeepSeekModels {
  models: string[];
  /** false when the documented fallback list was used (API unreachable) */
  fromApi: boolean;
}

export interface DeepSeekBalance {
  isAvailable: boolean;
  balances: { currency: string; totalBalance: string }[];
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
  documentUpdateGranularity: DocumentUpdateGranularity;
}

export type DiscussionMode = "debate" | "ideation" | "coConstruction" | "userDriven" | "socratic" | "tutorial" | "critiqueReview" | "collaborativeFiction";
export type DocumentFormat = "none" | "txt" | "md" | "csv";
export type DocumentInjectionMode = "rag" | "fullInjection";
/** Co-construction: regenerate the document once per turn (default) or after every intervention. */
export type DocumentUpdateGranularity = "turn" | "intervention";
/** Origin of a message's inner thought: in-character reflection or the model's raw reasoning. */
export type ThoughtKind = "persona" | "reasoning";

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
  thoughtKind?: ThoughtKind;
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
  /** Context window (Ollama) or context budget (DeepSeek), in tokens */
  numCtx: number;
  llmProvider: ProviderKind;
  reasoningLevel: ReasoningLevel;
  showModelReasoning: boolean;
  deepseekApiKey: string;
  deepseekModel: string;
  /** Monthly spending cap in USD (0 = unlimited) */
  deepseekMonthlyBudgetUsd: number;
  deepseekPeriodStart: string;
  deepseekPeriodUsageJson: string;
  deepseekUsageHistory: string;
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
  /** Serialised ArgumentMap (empty when disabled) */
  argumentMapJson: string;
  llmProvider: ProviderKind;
  usage: UsageLedger;
  estimatedCostUsd: number;
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
  llmProvider: string;
  totalTokens: number;
  estimatedCostUsd: number;
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
  /** Empty for discussions saved before v1.16 */
  argumentMapJson: string;
  llmProvider: string;
  usage: UsageLedger;
  estimatedCostUsd: number;
}

// ── Argument map (structured) ──────────────────────────────────────────

export type ArgumentType = "support" | "counter" | "evidence";

export interface ArgumentNode {
  id: string;
  label: string;
  argType: ArgumentType;
  speakerId: string;
  speakerName: string;
  targetsThesisId: string | null;
  children: ArgumentNode[];
}

export interface ThesisNode {
  id: string;
  label: string;
  speakerId: string;
  speakerName: string;
  arguments: ArgumentNode[];
}

export interface ArgumentMap {
  theses: ThesisNode[];
}

/** One undirected edge of the reaction graph (counts in both directions). */
export interface RelationshipEdge {
  a: string;
  b: string;
  abLikes: number;
  abDislikes: number;
  baLikes: number;
  baDislikes: number;
  /** "ally" | "rival" | "tense" when strong enough, else null */
  kind: string | null;
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
  | { type: "speakerPassed"; data: { speakerId: string; speakerName: string } }
  | { type: "relationshipsUpdated"; data: { edges: RelationshipEdge[] } }
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
        /** Participant addressed in priority (null = the topic) */
        focusSpeaker: string | null;
        reasoningLevel: ReasoningLevel;
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
        map: ArgumentMap;
        newNodeIds: string[];
        droppedCount: number;
      };
    }
  | {
      type: "llmUsageUpdated";
      data: {
        provider: ProviderKind;
        model: string;
        total: LlmUsage;
        calls: number;
        estimatedCostUsd: number | null;
        periodSpentUsd: number;
        budgetUsd: number;
        peak: boolean;
      };
    }
  | {
      type: "budgetAlert";
      data: { level: "warning" | "exceeded"; spentUsd: number; budgetUsd: number };
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
  focusSpeaker: string | null;
  reasoningLevel: ReasoningLevel;
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
  /** Provider whose tokenizer ratio applies */
  provider: ProviderKind;
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
  /** documentAvailableTokens expressed as printed pages */
  documentAvailablePages: number;
}
