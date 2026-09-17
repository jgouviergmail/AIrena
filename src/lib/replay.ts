// Replay of a discussion (v1.18): the messages come back at their real rhythm
// (deltas between timestamps), sped up by a factor and bounded so a long
// silence never freezes the player. Pure — tested in node.
import type { Message } from "./types";

/** Longest wait between two replayed messages, at ×1 (ms). */
export const REPLAY_MAX_GAP_MS = 8_000;
/** Shortest wait, so the feed stays readable even at ×8 (ms). */
export const REPLAY_MIN_GAP_MS = 250;
export const REPLAY_SPEEDS = [1, 2, 4, 8] as const;
export type ReplaySpeed = (typeof REPLAY_SPEEDS)[number];

export interface ReplayStep {
  /** Index of the message to reveal */
  index: number;
  /** When to reveal it, ms after the start of the replay */
  at: number;
}

/**
 * Schedule of a replay: the first message at once, each next one after the
 * real delta (capped at `REPLAY_MAX_GAP_MS`, then divided by `speed`, floored
 * at `REPLAY_MIN_GAP_MS`). A missing or unordered timestamp counts as no delay.
 */
export function scheduleReplay(messages: Pick<Message, "timestamp">[], speed: ReplaySpeed): ReplayStep[] {
  const steps: ReplayStep[] = [];
  let at = 0;
  let previous: number | null = null;
  messages.forEach((m, index) => {
    const time = Date.parse(m.timestamp);
    const stamp = Number.isNaN(time) ? null : time;
    if (index > 0) {
      const delta = previous !== null && stamp !== null ? Math.max(0, stamp - previous) : 0;
      at += Math.max(REPLAY_MIN_GAP_MS, Math.min(delta, REPLAY_MAX_GAP_MS) / speed);
    }
    steps.push({ index, at });
    if (stamp !== null) previous = stamp;
  });
  return steps;
}

/** Total duration of a schedule (ms). */
export function replayDuration(steps: ReplayStep[]): number {
  return steps.length ? steps[steps.length - 1].at : 0;
}
