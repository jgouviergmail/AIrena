// Mirror of Rust types — keep in sync with src-tauri/src/models/
import type { AgendaRecord, DiscussionDiagnostics, TurnTimings } from "./report";

// ── LLM provider ────────────────────────────────────────────────────────

/** Backend serving the discussion (settings.llmProvider). */
export type ProviderKind = "ollama" | "deepseek" | "openaiCompat";

/** Reasoning ("thinking") intensity. "auto" is resolved per call by the engine. */
export type ReasoningLevel = "off" | "low" | "high" | "max" | "auto";

export const REASONING_LEVELS: ReasoningLevel[] = ["auto", "off", "low", "high", "max"];

/** Wall-clock pace of the reasoning: "fast" caps Auto at Low and halves the DeepSeek allowances. */
export type ReasoningPace = "normal" | "fast";
export const REASONING_PACES: ReasoningPace[] = ["normal", "fast"];

/** How the voice keeps up with the discussion (v1.18). */
export type TtsMode = "follow" | "full";
export const TTS_MODES: TtsMode[] = ["follow", "full"];

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
  | "respondOrPass"
  | "intention"
  | "agenda"
  | "casting"
  | "recap"
  | "turnAnalyst"
  | "verdict"
  | "crisisDispatches"
  | "announcement"
  | "audienceQuestion";

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
  /** Suggested base URL of an OpenAI-compatible server (v1.20) */
  openaiCompatDefaultBaseUrl: string;
  /** Where releases are published (manual update check, v1.20) */
  releasesUrl: string;
}

/** Models an OpenAI-compatible server publishes (empty without catalogue). */
export interface OpenAiCompatModels {
  models: string[];
}

/** Engine limits the frontend mirrors in its UI (owned by the backend). */
export interface EngineConstants {
  /** Audience reactions accepted per message */
  audienceReactionsPerMessageMax: number;
  /** Chars the secret agenda block takes in the system prompt (budget preview) */
  agendaBlockMaxChars: number;
  /** Gladiateurs a casting may suggest, at most */
  castingMaxGladiateurs: number;
}

/** Casting suggested by the model for a topic (ids of the profile catalogue). */
export interface CastingSuggestion {
  gladiateurs: { id: string; reason: string }[];
  arbitre: string | null;
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
  /** Role in the structured modes (trial, Oxford); the engine deals the missing ones (v1.19) */
  modeRole?: string;
  /** Model of this speaker (undefined = the provider's global model, v1.20) */
  model?: string;
}

export interface IArbitreConfig {
  /** Model of the moderator (undefined = the provider's global model, v1.20) */
  model?: string;
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
  features: DiscussionFeatures;
}

/** When participants react: right after each intervention, or to the previous turn before speaking (v1.16). */
export type ReactionTiming = "immediate" | "deferred";

/** Liveliness options of a discussion (mirror of `DiscussionFeatures`). */
export interface DiscussionFeatures {
  reactionTiming: ReactionTiming;
  audienceReactions: boolean;
  sceneEvents: boolean;
  hiddenAgenda: boolean;
  coalitions: boolean;
}

export const DEFAULT_DISCUSSION_FEATURES: DiscussionFeatures = {
  reactionTiming: "immediate",
  audienceReactions: true,
  sceneEvents: true,
  hiddenAgenda: true,
  coalitions: true,
};

export type DiscussionMode =
  | "debate" | "ideation" | "coConstruction" | "userDriven" | "socratic" | "tutorial" | "critiqueReview" | "collaborativeFiction"
  | "trial" | "oxfordDebate" | "negotiation" | "sixHats" | "crisisCell";
export type DocumentFormat = "none" | "txt" | "md" | "csv";
export type DocumentInjectionMode = "rag" | "fullInjection";
/** Co-construction: regenerate the document once per turn (default) or after every intervention. */
export type DocumentUpdateGranularity = "turn" | "intervention";
/** Origin of a message's inner thought: in-character reflection or the model's raw reasoning. */
export type ThoughtKind = "persona" | "reasoning";

export type SpeakerRole = "IArbitre" | "GladIAteur" | "user";
/** like/dislike are the historical pair; the others refine them (v1.17). */
export type ReactionType = "like" | "dislike" | "insightful" | "question" | "offTopic" | "laugh";
export const REACTION_TYPES: ReactionType[] = ["like", "dislike", "insightful", "question", "offTopic", "laugh"];
/** Approval / disapproval classes (neutral otherwise) — mirrors the Rust classification. */
export const POSITIVE_REACTIONS: ReadonlySet<ReactionType> = new Set(["like", "insightful"]);
export const NEGATIVE_REACTIONS: ReadonlySet<ReactionType> = new Set(["dislike", "offTopic"]);

/** What a message is (contribution or one of the engine's system lines). */
export type MessageKind = "normal" | "banNotification" | "stageDirection" | "actAnnouncement" | "sceneEvent";

export interface Reaction {
  fromSpeakerId: string;
  fromSpeakerName: string;
  reactionType: ReactionType;
  targetMessageId: string;
  justification?: string;
  /** Exact excerpt of the target message (validated by the engine) */
  quote?: string;
  /** Frontend only: audience reaction sent, not yet confirmed by the engine */
  pending?: boolean;
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
  /** Kept for v1.16 payloads; always consistent with kind === "banNotification" */
  isBanNotification: boolean;
  /** Absent on v1.16 payloads (= "normal", or "banNotification" when the flag is set) */
  kind?: MessageKind;
  timestamp: string;
}

/** Message kind with the v1.16 fallback applied. */
export function messageKind(m: Pick<Message, "kind" | "isBanNotification">): MessageKind {
  return m.kind ?? (m.isBanNotification ? "banNotification" : "normal");
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
  reasoningPace: ReasoningPace;
  ttsEnabled: boolean;
  ttsMode: TtsMode;
  ttsVolume: number;
  soundEnabled: boolean;
  soundVolume: number;
  deepseekApiKey: string;
  deepseekModel: string;
  /** Monthly spending cap in USD (0 = unlimited) */
  deepseekMonthlyBudgetUsd: number;
  deepseekPeriodStart: string;
  deepseekPeriodUsageJson: string;
  deepseekUsageHistory: string;
  /** OpenAI-compatible server (v1.20): base URL, optional key, model, manual model list (JSON array) */
  openaiCompatBaseUrl: string;
  openaiCompatApiKey: string;
  openaiCompatModel: string;
  openaiCompatModels: string;
  /** Advanced tuning override (JSON of the knobs that differ from the defaults, v1.20) */
  advancedTuningJson: string;
  /** Personas remember their past discussions (v1.20) */
  personaMemoryEnabled: boolean;
}

/** What a persona keeps of a discussion (long memory, v1.20) — mirror of `PersonaRecap`. */
export interface PersonaRecap {
  positions: string[];
  bestLines: string[];
  allies: string[];
  rivals: string[];
  lesson: string;
}

export interface PersonaRecapRecord {
  speakerId: string;
  speakerName: string;
  profileId: string;
  recap: PersonaRecap;
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
  /** Serialised DiscussionReport (lib/report.ts); empty when nothing was collected */
  reportJson: string;
  /** Recaps of the personas built from a catalogue profile (persisted as their long memory) */
  recaps: PersonaRecapRecord[];
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
  /** User tags and favourite flag (v1.20) */
  tags: string[];
  favorite: boolean;
}

/** A reusable discussion setup (v1.20). `configJson` holds a `TemplateConfig` (`lib/templates.ts`). */
export interface DiscussionTemplate {
  id: string;
  name: string;
  configJson: string;
  /** Seeded templates cannot be deleted; their name is translated from `templates.<id>` */
  builtin: boolean;
  createdAt: string;
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
  /** Empty for discussions saved before v1.17 */
  reportJson: string;
  tags: string[];
  favorite: boolean;
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

/** How deep the argument map goes (v1.20.1). */
export interface ArgumentMapDepth {
  maxDepth: number;
  /** Share of arguments answering another argument (depth ≥ 2), 0–1 */
  deepShare: number;
  /** Counter-arguments nobody has answered yet */
  unansweredCounters: number;
}

export interface ArgumentMap {
  theses: ThesisNode[];
}

/** One undirected edge of the reaction graph (counts in both directions). */
/** How an edge's net warmth moved since the previous update (v1.17). */
export type RelationshipTrend = "warming" | "cooling" | "stable";

export interface RelationshipEdge {
  a: string;
  b: string;
  abLikes: number;
  abDislikes: number;
  baLikes: number;
  baDislikes: number;
  /** "ally" | "rival" | "tense" when strong enough, else null */
  kind: string | null;
  /** Net weighted warmth (approvals − disapprovals, both ways, decayed each turn) */
  score?: number;
  trend?: RelationshipTrend | null;
}

/** Temperature of the room (average of the active gladiateurs, v1.17). */
export type RoomMood = "tense" | "flat" | "lively" | "serene";

/** Acts of the mode scripts (v1.18) — mirror of `engine::dramaturgy::ActKey`. */
export type ActKey =
  | "opening" | "confrontation" | "crossExamination" | "concessions" | "closingStatements"
  | "divergence" | "association" | "convergence"
  | "proposal" | "critique" | "consolidation"
  | "questioning" | "digging" | "synthesis"
  | "foundations" | "deepening" | "recap"
  | "impressions" | "examination" | "recommendations"
  | "exposition" | "complication" | "climax" | "resolution";

export type FormatConstraintKind = "oneSentence" | "noJargon" | "metaphor" | "numbers" | "endWithQuestion";

/** A scene event of the turn (v1.18) — mirror of `engine::scene_events::SceneEvent`. */
export type SceneEvent =
  | { kind: "surpriseFact"; fact: string; source: string | null }
  | { kind: "formatConstraint"; constraint: FormatConstraintKind }
  | { kind: "audienceQuestion"; target: string; question: string }
  | { kind: "forcedSteelman" }
  | { kind: "duel"; a: string; b: string }
  | { kind: "hotSeat"; target: string }
  | { kind: "dispatch"; text: string; index: number; total: number };

/** Who plays what this turn (v1.19) — mirror of `RoleAssignment`. */
export interface RoleAssignment {
  speakerId: string;
  role: string;
  /** Display label in the discussion language */
  label: string;
}

/** When the audience votes in an Oxford debate. */
export type VotePhase = "before" | "after";

export interface VerdictVote {
  voterId: string;
  voterName: string;
  choice: string;
  reason: string;
}

export interface PartyDecision {
  partyId: string;
  partyName: string;
  accepts: boolean;
  reason: string;
}

/** Mode-specific result (v1.19) — mirror of `ModeOutcome`, persisted in `report_json.outcome`. */
export type ModeOutcome =
  | { kind: "verdict"; votes: VerdictVote[]; winner: string | null; byArbitre: boolean }
  | { kind: "agreement"; parties: PartyDecision[]; reached: boolean }
  | { kind: "audienceSwing"; before: string | null; after: string | null; winner: string | null };

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
  | { type: "intentionGenerated"; data: { speakerId: string; speakerName: string } & IntentionData }
  | { type: "relationshipShift"; data: { a: string; b: string; from: string; to: string } }
  | { type: "roomMoodUpdated"; data: { avg: EmotionalProfile; label: RoomMood } }
  | { type: "turnTimings"; data: { timings: TurnTimings } }
  | { type: "actStarted"; data: { turn: number; act: ActKey; title: string } }
  | { type: "sceneEventTriggered"; data: { turn: number; event: SceneEvent; participants: string[] } }
  | { type: "coalitionFormed"; data: { turn: number; a: string; b: string; aName: string; bName: string } }
  | { type: "diagnosticsReady"; data: { diagnostics: DiscussionDiagnostics } }
  | { type: "agendaRevealed"; data: { agendas: AgendaRecord[] } }
  | { type: "rolesAssigned"; data: { turn: number; roles: RoleAssignment[] } }
  | { type: "audienceVoteRequested"; data: { phase: VotePhase; timeoutSecs: number } }
  | { type: "audienceVoteRecorded"; data: { phase: VotePhase; choice: string } }
  | { type: "outcomeReady"; data: { outcome: ModeOutcome } }
  | { type: "personaRecapReady"; data: { recap: PersonaRecapRecord } }
  | { type: "positionsUpdated"; data: { positions: ParticipantPosition[] } }
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
  | { type: "awaitingCue"; data: { speakerId: string; speakerName: string } }
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
        /** Results injected into the prompt (v1.17) */
        results: WebSourceInfo[];
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
        /** Articles injected into the prompt (v1.17) */
        articles: WikiSourceInfo[];
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
        /** Depth of the map and objections waiting for an answer (v1.20.1; absent in older fixtures) */
        depth?: ArgumentMapDepth;
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

// ── Sources ────────────────────────────────────────────────────────────

/** One web result injected into a prompt. */
export interface WebSourceInfo {
  title: string;
  url: string;
  domain: string;
  snippet: string;
}

/** One Wikipedia article injected into a prompt. */
export interface WikiSourceInfo {
  title: string;
  url: string;
  snippet: string;
}

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

/** What a speaker tries to achieve with their coming intervention (v1.17). */
export type IntentionGoal = "convince" | "nuance" | "contest" | "question" | "concede" | "relaunch";

/** Pre-speech contract of a speaker, shown in the backstage panel. */
export interface IntentionData {
  /** Participant addressed (display name); null = the topic */
  target: string | null;
  goal: IntentionGoal;
  angle: string;
  concession: string | null;
  question: string | null;
}

/** A participant's position and its trajectory (engine memory, v1.17). */
export interface ParticipantPosition {
  participantName: string;
  stance: string;
  initialStance?: string | null;
  shift?: string | null;
  wouldChangeIf?: string | null;
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
  | "positionalMap"
  | "openLoops"
  | "debateState";

/** Sections whose priority rank can be configured by the user in Settings. */
export const CONFIGURABLE_BUDGET_SECTIONS: BudgetSection[] = [
  "currentTurnMessages",
  "immediateMemory",
  "contextualSummary",
  "cognitiveDirectives",
  "arbitreDirectives",
  "webWikiSearch",
  "positionalMap",
  "openLoops",
  "debateState",
];

/** Configurable sections added after v1.16: a saved order without them stays valid (they are appended). */
export const LATER_BUDGET_SECTIONS: BudgetSection[] = ["openLoops", "debateState"];

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
  /** Chars of the hidden agenda block in the system prompt (0 when off) */
  agendaChars: number;
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
