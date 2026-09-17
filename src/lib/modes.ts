// Discussion modes (v1.19): which ones take a hidden agenda, which ones deal
// roles, the audience's vote choices — mirrors of the Rust side (`DiscussionMode`,
// `engine::mode_roles`, `models::outcome`).
import type { DiscussionMode } from "./types";

/** Every mode, in the order the wizard shows them. */
export const DISCUSSION_MODES: DiscussionMode[] = [
  "debate", "ideation", "coConstruction", "userDriven",
  "socratic", "tutorial", "critiqueReview", "collaborativeFiction",
  "trial", "oxfordDebate", "negotiation", "sixHats", "crisisCell",
];

/** Modes where a secret agenda makes sense (mirrors `DiscussionMode::supports_hidden_agenda`). */
export const HIDDEN_AGENDA_MODES: DiscussionMode[] = ["debate", "collaborativeFiction", "trial", "oxfordDebate", "negotiation"];

export function modeSupportsHiddenAgenda(mode: DiscussionMode): boolean {
  return HIDDEN_AGENDA_MODES.includes(mode);
}

/** Roles the user may assign per gladiateur (the engine deals the missing ones from the casting order). */
export const SELECTABLE_ROLES: Partial<Record<DiscussionMode, readonly string[]>> = {
  trial: ["prosecutor", "defense", "witness", "juror"],
  oxfordDebate: ["for", "against"],
};

export function selectableRoles(mode: DiscussionMode): readonly string[] {
  return SELECTABLE_ROLES[mode] ?? [];
}

/** Modes that force the sequential order (the engine ignores the distribution). */
export const FIXED_ORDER_MODES: DiscussionMode[] = ["userDriven", "collaborativeFiction"];

/** The audience's choices on an Oxford motion. */
export type VoteChoice = "for" | "against";
export const VOTE_CHOICES: VoteChoice[] = ["for", "against"];
