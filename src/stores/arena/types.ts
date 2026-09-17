// Data part of the arena store (no actions). Reducers in ./reducers/*.ts
// update slices of it from ArenaEvents; useArenaStore orchestrates.
import type { AgendaRecord, DiscussionDiagnostics, PositionRecord, SourceRecord, TimelineEntry, TurnTimings } from "@/lib/report";
import type { PendingSource } from "@/lib/sources";
import type {
  ActKey,
  ArenaEvent,
  ModeOutcome,
  PersonaRecapRecord,
  RoleAssignment,
  VotePhase,
  ArgumentMap,
  ArgumentMapDepth,
  BanInfo,
  DirectiveData,
  DocumentFormat,
  EmotionalProfile,
  EmotionSnapshot,
  IntentionData,
  LlmUsage,
  Message,
  ProviderKind,
  RagChunkInfo,
  ReactionType,
  RelationshipEdge,
  RoomMood,
  SceneEvent,
} from "@/lib/types";

/** A reaction flying from its author to the target on the stage (short-lived). */
export interface ReactionBurst {
  id: string;
  from: string;
  to: string;
  reactionType: ReactionType;
}

/** A full-width announcement shown for `BANNER_MS` (turn, act, scene event, ban). */
export interface StageBanner {
  id: string;
  kind: "turn" | "act" | "sceneEvent" | "ban";
  /** Turn number, act title, scene event kind or banned name */
  text: string;
  speakerId?: string;
}

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

/** Real-time engine activity for the status bar */
export interface ActivityStatus {
  type: "thinking" | "writing" | "reacting" | "webSearch" | "wikiSearch" | "ragInjection" | "ragCacheHit" | "emotions" | "argumentMap" | "synthesis" | "determining";
  speakerName?: string;
}

export type ArenaStatus = "idle" | "running" | "paused" | "synthesizing" | "ended";

export interface ArenaData {
  discussionId: string | null;
  status: ArenaStatus;
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
  /** Step mode: the engine waits for the audience's cue before this speaker talks (v1.20.1) */
  awaitingCue: { speakerId: string; speakerName: string } | null;
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
  /** Depth of the map and open objections (v1.20.1) */
  argumentMapDepth: ArgumentMapDepth | null;
  directives: Map<string, DirectiveData>;
  /** Latest pre-speech contract per speaker (backstage) */
  intentions: Map<string, IntentionData>;
  /** Concessions granted per speaker (from their intentions), for the score */
  concessions: Record<string, number>;
  /** Positions and their trajectory after the last memory update */
  positions: PositionRecord[];
  bans: Map<string, BanInfo>;
  /** UserDriven: participants who passed this turn (in order) */
  passedSpeakerIds: string[];
  /** Cumulative reaction graph (relations tab) */
  relationships: RelationshipEdge[];
  /** Temperature of the room after the last emotion round (null below two active speakers) */
  roomMood: { avg: EmotionalProfile; label: RoomMood } | null;
  llmUsage: LlmUsageStatus | null;
  budgetAlert: BudgetAlertInfo | null;
  lastThresholdCrossed: ThresholdCrossing | null;
  /** References injected into prompts, attached to the message they served */
  sources: SourceRecord[];
  /** Wall-clock phases of every finished turn */
  timings: TurnTimings[];
  /** Act of the mode's script in progress (null in UserDriven) */
  currentAct: { key: ActKey; title: string; turn: number } | null;
  /** Scene event of the current turn (cleared when a turn starts) */
  sceneEvent: { turn: number; event: SceneEvent; participants: string[] } | null;
  /** Coalition of the current turn: leader and follower ids */
  coalition: { a: string; b: string } | null;
  /** Floating reactions on the stage (capped, dismissed by the stage) */
  bursts: ReactionBurst[];
  /** Latest banner to show (dismissed by the banner component) */
  banner: StageBanner | null;
  /** Notable moments (acts, scene events, coalitions, bans, shifts, user turns) */
  timeline: TimelineEntry[];
  /** Engine diagnostics, received just before the end */
  diagnostics: DiscussionDiagnostics | null;
  /** Secret agendas unveiled after the synthesis (v1.19) */
  agendas: AgendaRecord[];
  /** Roles or hats of the turn, by speaker id (v1.19) */
  roles: Map<string, RoleAssignment>;
  /** Open audience vote window (Oxford), cleared once the vote lands or the debate moves on */
  voteRequest: { phase: VotePhase; timeoutSecs: number } | null;
  /** The audience's votes so far (Oxford) */
  votes: Partial<Record<VotePhase, string>>;
  /** Mode-specific result (verdict, agreement, audience swing) */
  outcome: ModeOutcome | null;
  /** What each persona keeps of the discussion (saved with it as long memory, v1.20) */
  recaps: PersonaRecapRecord[];
  _pendingSources: PendingSource[];
  error: string | null;
}

/** Fresh data state. A function: Maps must never be shared between resets. */
export function initialArenaData(): ArenaData {
  return {
    discussionId: null,
    status: "idle",
    currentTurn: 0,
    speakerOrder: [],
    activeSpeakerId: null,
    activityStatus: null,
    messages: [],
    emotions: new Map(),
    emotionHistory: new Map(),
    moodSummary: new Map(),
    synthesis: "",
    synthesisStreaming: "",
    userTurnActive: false,
    interventionRequested: false,
    awaitingCue: null,
    determiningOrder: false,
    webSearchCount: 0,
    _pendingSearchCount: 0,
    webSearchesPerMessage: {},
    wikiSearchCount: 0,
    _pendingWikiCount: 0,
    wikiSearchesPerMessage: {},
    wikiArticleUrlsPerMessage: {},
    _pendingWikiUrls: [],
    ragChunkCount: 0,
    _pendingRagCount: 0,
    _pendingRagChunks: [],
    ragChunksPerMessage: {},
    ragChunkDetailsPerMessage: {},
    documentContent: "",
    previousDocumentContent: null,
    documentFormat: "none",
    documentLastEditor: null,
    argumentMapMarkdown: "",
    argumentMapMarkdownBySpeaker: "",
    argumentMapThesesCount: 0,
    argumentMapArgumentsCount: 0,
    argumentMapDepth: null,
    argumentMap: null,
    argumentMapNewNodeIds: [],
    argumentMapDroppedCount: 0,
    directives: new Map(),
    intentions: new Map(),
    concessions: {},
    positions: [],
    bans: new Map(),
    passedSpeakerIds: [],
    relationships: [],
    roomMood: null,
    llmUsage: null,
    budgetAlert: null,
    lastThresholdCrossed: null,
    sources: [],
    timings: [],
    currentAct: null,
    sceneEvent: null,
    coalition: null,
    bursts: [],
    banner: null,
    timeline: [],
    diagnostics: null,
    agendas: [],
    roles: new Map(),
    voteRequest: null,
    votes: {},
    outcome: null,
    recaps: [],
    _pendingSources: [],
    error: null,
  };
}

/**
 * A reducer handles the events of one domain and returns the slice to merge,
 * or null when the event is not its concern. Pure: no side effects, no
 * module-level state — testable in node without a DOM.
 */
export type ArenaReducer = (state: ArenaData, event: ArenaEvent) => Partial<ArenaData> | null;
