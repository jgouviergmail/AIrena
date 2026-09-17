// History filters (v1.20): pure narrowing of the discussion list by mode,
// provider, participant, tag and favourite. The full-text search itself is
// done by the backend (`search_discussion_history`).
import type { DiscussionSummary } from "./types";

export interface HistoryFilters {
  mode: string | null;
  provider: string | null;
  /** Case-insensitive fragment of a participant name */
  participant: string;
  tag: string | null;
  favoriteOnly: boolean;
}

export const EMPTY_FILTERS: HistoryFilters = { mode: null, provider: null, participant: "", tag: null, favoriteOnly: false };

export function hasActiveFilters(f: HistoryFilters): boolean {
  return f.mode !== null || f.provider !== null || f.participant.trim().length > 0 || f.tag !== null || f.favoriteOnly;
}

export function filterDiscussions(list: DiscussionSummary[], f: HistoryFilters): DiscussionSummary[] {
  const participant = f.participant.trim().toLowerCase();
  return list.filter((d) =>
    (f.mode === null || d.discussionMode === f.mode)
    && (f.provider === null || d.llmProvider === f.provider)
    && (participant.length === 0 || d.participants.some((p) => p.name.toLowerCase().includes(participant)))
    && (f.tag === null || d.tags.includes(f.tag))
    && (!f.favoriteOnly || d.favorite),
  );
}

/** Every tag in use, most frequent first (ties alphabetical). */
export function collectTags(list: DiscussionSummary[]): string[] {
  const counts = new Map<string, number>();
  for (const d of list) for (const tag of d.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
  return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([tag]) => tag);
}

/** Normalise a tag typed by the user: trimmed, lower-cased, no leading '#', bounded. */
export const TAG_MAX_LENGTH = 24;
export function normaliseTag(raw: string): string {
  return raw.trim().replace(/^#+/, "").toLowerCase().slice(0, TAG_MAX_LENGTH);
}
