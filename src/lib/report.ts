// Analytical report of a discussion, persisted as `discussions.report_json`.
// Built by the arena store from the events it received; every field has a
// default so that older or partial reports still parse.
import type { EmotionSnapshot, ModeOutcome, RelationshipEdge } from "./types";

export const REPORT_VERSION = 1;

/** Where a source came from. */
export type SourceKind = "web" | "wiki" | "rag";

/** One reference injected into a speaker's prompt (web result, Wikipedia article, RAG chunk). */
export interface SourceRecord {
  kind: SourceKind;
  turn: number;
  speakerId: string;
  speakerName: string;
  /** Message the source was attached to (null when the message never completed) */
  messageId: string | null;
  title: string;
  /** http(s) URL for web/wiki; "file#index" for RAG chunks */
  url: string;
  snippet: string;
  /** Heuristic: the speaker's message mentions the domain, the title or a distinctive part of the snippet */
  cited: boolean;
}

export type TimelineEntryKind = "turn" | "act" | "sceneEvent" | "ban" | "banLifted" | "threshold" | "coalition" | "relationshipShift" | "userTurn";

/** A notable moment (for the timeline bar and the replay). */
export interface TimelineEntry {
  kind: TimelineEntryKind;
  turn: number;
  /** Message index the moment is anchored to (for scrolling), when applicable */
  messageIndex: number | null;
  label: string;
  speakerId?: string;
}

export interface AgendaRecord {
  speakerId: string;
  speakerName: string;
  objective: string;
  redLine: string;
  victory: string;
  /** Filled by the synthesis: did they get there? */
  achieved: boolean | null;
}

export interface PositionRecord {
  participantName: string;
  stance: string;
  initialStance: string | null;
  shift: string | null;
  wouldChangeIf: string | null;
}

export interface Award {
  key: string;
  speakerId: string;
  speakerName: string;
  /** Free-form evidence (a quote, a count) shown under the trophy */
  detail: string;
}

export interface PhaseTiming {
  name: string;
  ms: number;
}

export interface TurnTimings {
  turn: number;
  phases: PhaseTiming[];
}

export interface DiscussionDiagnostics {
  jsonParseFailures: Record<string, number>;
  refusals: number;
  retries: number;
  /** Share of interventions that addressed the target their intention named (0–1, null when unknown) */
  intentionCompliance: number | null;
}

export interface DiscussionReport {
  version: number;
  sources: SourceRecord[];
  timeline: TimelineEntry[];
  emotionHistory: Record<string, EmotionSnapshot[]>;
  relationships: RelationshipEdge[];
  positions: PositionRecord[];
  agendas: AgendaRecord[];
  awards: Award[];
  /** Mode-specific result (trial verdict, negotiation agreement, audience swing) */
  outcome: ModeOutcome | null;
  timings: TurnTimings[];
  diagnostics: DiscussionDiagnostics | null;
}

export function emptyReport(): DiscussionReport {
  return {
    version: REPORT_VERSION,
    sources: [],
    timeline: [],
    emotionHistory: {},
    relationships: [],
    positions: [],
    agendas: [],
    awards: [],
    outcome: null,
    timings: [],
    diagnostics: null,
  };
}

/** True when the report carries anything worth showing. */
export function reportHasContent(r: DiscussionReport): boolean {
  return r.sources.length > 0 || r.timeline.length > 0 || Object.keys(r.emotionHistory).length > 0
    || r.positions.length > 0 || r.agendas.length > 0 || r.awards.length > 0 || r.outcome !== null;
}

/** Parse a persisted report; empty, malformed or unknown → defaults (older discussions). */
export function parseReportJson(json: string | null | undefined): DiscussionReport {
  const base = emptyReport();
  if (!json) return base;
  try {
    const parsed = JSON.parse(json) as Partial<DiscussionReport> | null;
    if (!parsed || typeof parsed !== "object") return base;
    return {
      version: typeof parsed.version === "number" ? parsed.version : REPORT_VERSION,
      sources: Array.isArray(parsed.sources) ? parsed.sources : [],
      timeline: Array.isArray(parsed.timeline) ? parsed.timeline : [],
      emotionHistory: parsed.emotionHistory && typeof parsed.emotionHistory === "object" ? parsed.emotionHistory : {},
      relationships: Array.isArray(parsed.relationships) ? parsed.relationships : [],
      positions: Array.isArray(parsed.positions) ? parsed.positions : [],
      agendas: Array.isArray(parsed.agendas) ? parsed.agendas : [],
      awards: Array.isArray(parsed.awards) ? parsed.awards : [],
      outcome: parsed.outcome && typeof parsed.outcome === "object" && typeof parsed.outcome.kind === "string" ? parsed.outcome : null,
      timings: Array.isArray(parsed.timings) ? parsed.timings : [],
      diagnostics: parsed.diagnostics && typeof parsed.diagnostics === "object" ? parsed.diagnostics : null,
    };
  } catch {
    return base;
  }
}

/** Serialise for persistence; empty string when there is nothing to keep (keeps the DB small). */
export function serializeReport(r: DiscussionReport): string {
  return reportHasContent(r) ? JSON.stringify(r) : "";
}
