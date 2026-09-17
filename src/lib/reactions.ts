import { NEGATIVE_REACTIONS, POSITIVE_REACTIONS, type ReactionType } from "./types";

/** Emoji shown on a reaction chip / button. */
export const REACTION_EMOJI: Record<ReactionType, string> = {
  like: "👍",
  dislike: "👎",
  insightful: "💡",
  question: "❓",
  offTopic: "🚫",
  laugh: "😂",
};

export type ReactionTone = "positive" | "negative" | "neutral";

/** Approval class of a reaction (drives colours and the relationship graph). */
export function reactionTone(type: ReactionType): ReactionTone {
  if (POSITIVE_REACTIONS.has(type)) return "positive";
  if (NEGATIVE_REACTIONS.has(type)) return "negative";
  return "neutral";
}

/** Tailwind classes of a reaction chip by tone. */
export const REACTION_TONE_CLASS: Record<ReactionTone, string> = {
  positive: "bg-green-500/10 text-green-500",
  negative: "bg-red-500/10 text-red-500",
  neutral: "bg-blue-500/10 text-blue-500",
};
