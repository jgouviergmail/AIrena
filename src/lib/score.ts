// Score of the arena (v1.18): points per turn and a ranking from what the
// participants drew (reactions), built (arguments on the map), granted
// (concessions) and suffered (bans); the awards of the closing credits. Pure
// functions over the store's state — tested in node.
import type { Award, PositionRecord, TimelineEntry } from "./report";
import type { ArgumentMap, ArgumentNode, BanInfo, Message, ReactionType } from "./types";

/** Points a received reaction is worth. */
export const REACTION_POINTS: Record<ReactionType, number> = {
  like: 2,
  insightful: 3,
  laugh: 1,
  question: 1,
  dislike: -1,
  offTopic: -2,
};
/** Points per thesis and per argument added to the map. */
export const THESIS_POINTS = 3;
export const ARGUMENT_POINTS = 1;
/** Points per concession granted. */
export const CONCESSION_POINTS = 2;
/** Points lost per ban. */
export const BAN_POINTS = -5;

export interface ScoreRow {
  speakerId: string;
  speakerName: string;
  /** Total points */
  points: number;
  /** Points per turn (index = turn number) */
  byTurn: Record<number, number>;
  reactions: number;
  arguments: number;
  concessions: number;
  bans: number;
  /** 1-based rank; equal points share the rank */
  rank: number;
}

export interface ScoreInputs {
  messages: Message[];
  argumentMap: ArgumentMap | null;
  bans: Map<string, BanInfo>;
  /** Concessions granted per speaker id (from their intentions) */
  concessions: Record<string, number>;
  /** Gladiateurs to rank (id → name); speakers absent from the list are ignored */
  participants: { id: string; name: string }[];
}

function countArguments(nodes: ArgumentNode[], speakerId: string): number {
  return nodes.reduce((n, a) => n + (a.speakerId === speakerId ? 1 : 0) + countArguments(a.children, speakerId), 0);
}

/** Reaction points of one message. */
export function messagePoints(message: Message): number {
  return message.reactions.reduce((sum, r) => sum + (REACTION_POINTS[r.reactionType] ?? 0), 0);
}

/** Scores and ranking of the gladiateurs. */
export function computeScores(inputs: ScoreInputs): ScoreRow[] {
  const rows = inputs.participants.map((p) => {
    const byTurn: Record<number, number> = {};
    let reactions = 0;
    for (const m of inputs.messages) {
      if (m.speakerId !== p.id || (m.kind ?? "normal") !== "normal") continue;
      const pts = messagePoints(m);
      reactions += pts;
      byTurn[m.turnNumber] = (byTurn[m.turnNumber] ?? 0) + pts;
    }
    const theses = inputs.argumentMap?.theses.filter((t) => t.speakerId === p.id).length ?? 0;
    const args = inputs.argumentMap?.theses.reduce((n, t) => n + countArguments(t.arguments, p.id), 0) ?? 0;
    const argumentsPoints = theses * THESIS_POINTS + args * ARGUMENT_POINTS;
    const concessions = (inputs.concessions[p.id] ?? 0) * CONCESSION_POINTS;
    const bans = (inputs.bans.get(p.id)?.totalBans ?? 0) * BAN_POINTS;
    return {
      speakerId: p.id,
      speakerName: p.name,
      points: reactions + argumentsPoints + concessions + bans,
      byTurn,
      reactions,
      arguments: argumentsPoints,
      concessions,
      bans,
      rank: 0,
    };
  });
  rows.sort((a, b) => b.points - a.points || a.speakerName.localeCompare(b.speakerName));
  let rank = 0;
  let previous: number | null = null;
  rows.forEach((row, i) => {
    if (previous === null || row.points !== previous) rank = i + 1;
    row.rank = rank;
    previous = row.points;
  });
  return rows;
}

export interface AwardInputs extends ScoreInputs {
  positions: PositionRecord[];
  timeline: TimelineEntry[];
}

/** Excerpt shown under a trophy. */
/** Chars of the winning sentence quoted on an award (whole sentences, no ellipsis on a short one). */
const AWARD_DETAIL_MAX_CHARS = 280;

function excerpt(text: string, max = AWARD_DETAIL_MAX_CHARS): string {
  const clean = text.replace(/\s+/g, " ").trim();
  return clean.length > max ? `${clean.slice(0, max - 1)}…` : clean;
}

/**
 * The closing credits: best argument (the message that drew the most points),
 * biggest shift (a position that moved), most contested (most disapprovals),
 * most prolific (most theses and arguments), reconciliation (a rivalry that
 * ended in an approval). Only the awards that have a winner are returned.
 */
export function computeAwards(inputs: AwardInputs): Award[] {
  const scores = computeScores(inputs);
  const name = (id: string) => inputs.participants.find((p) => p.id === id)?.name ?? id;
  const awards: Award[] = [];

  const spoken = inputs.messages.filter((m) => (m.kind ?? "normal") === "normal" && inputs.participants.some((p) => p.id === m.speakerId));
  const best = spoken.map((m) => ({ m, pts: messagePoints(m) })).filter((x) => x.pts > 0).sort((a, b) => b.pts - a.pts)[0];
  if (best) awards.push({ key: "bestArgument", speakerId: best.m.speakerId, speakerName: best.m.speakerName, detail: excerpt(best.m.content) });

  const moved = inputs.positions.filter((p) => p.shift && (p.initialStance ?? p.stance) !== p.stance);
  const shift = moved[0] ?? inputs.positions.find((p) => p.shift);
  if (shift) {
    const who = inputs.participants.find((p) => p.name === shift.participantName);
    if (who) awards.push({ key: "biggestShift", speakerId: who.id, speakerName: who.name, detail: excerpt(`${shift.initialStance ?? ""} → ${shift.stance}`) });
  }

  const contested = inputs.participants
    .map((p) => ({ p, n: spoken.filter((m) => m.speakerId === p.id).reduce((n, m) => n + m.reactions.filter((r) => r.reactionType === "dislike" || r.reactionType === "offTopic").length, 0) }))
    .filter((x) => x.n > 0)
    .sort((a, b) => b.n - a.n)[0];
  if (contested) awards.push({ key: "mostContested", speakerId: contested.p.id, speakerName: contested.p.name, detail: String(contested.n) });

  const prolific = scores.filter((s) => s.arguments > 0).sort((a, b) => b.arguments - a.arguments)[0];
  if (prolific) awards.push({ key: "mostProlific", speakerId: prolific.speakerId, speakerName: prolific.speakerName, detail: String(prolific.arguments) });

  const reconciliation = inputs.timeline.find((e) => e.kind === "relationshipShift" && e.label.startsWith("rival →") && e.speakerId);
  if (reconciliation?.speakerId) awards.push({ key: "reconciliation", speakerId: reconciliation.speakerId, speakerName: name(reconciliation.speakerId), detail: reconciliation.label });

  return awards;
}
