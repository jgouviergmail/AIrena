// Pure helpers of the arena stage (v1.18): layout of the participants on an
// arc, dominant emotion of a profile (aura colour), timings of the effects and
// the keyboard shortcut mapping. No DOM here — tested in node.
import type { EmotionalProfile } from "./types";

/** Floating reactions kept on screen at once. */
export const STAGE_MAX_BURSTS = 6;
/** Life of a floating reaction (ms). */
export const BURST_MS = 900;
/** Life of a full-width banner (ms). */
export const BANNER_MS = 800;

export type EmotionAxis = keyof EmotionalProfile;

/** Hue of each emotional axis (aura colour). */
export const EMOTION_HUE: Record<EmotionAxis, number> = {
  engagement: 210,
  accord: 140,
  confiance: 45,
  frustration: 10,
  curiosite: 270,
  enthousiasme: 320,
};

const NEUTRAL = 50;

/**
 * The axis furthest from neutral (frustration counts from its low baseline),
 * with its distance — `null` when the profile is flat.
 */
export function dominantEmotion(profile: EmotionalProfile): { axis: EmotionAxis; intensity: number } | null {
  let best: { axis: EmotionAxis; intensity: number } | null = null;
  for (const axis of Object.keys(EMOTION_HUE) as EmotionAxis[]) {
    const value = profile[axis];
    const distance = axis === "frustration" ? Math.max(0, value - 10) : Math.abs(value - NEUTRAL);
    if (distance > 0 && (!best || distance > best.intensity)) best = { axis, intensity: distance };
  }
  return best;
}

export interface StagePoint {
  x: number;
  y: number;
}

/**
 * Positions of `count` participants on an arc that opens toward the bottom
 * (the audience). One participant sits at the top centre; the arc widens as
 * the cast grows, within the box.
 */
export function arcPositions(count: number, width: number, height: number): StagePoint[] {
  if (count <= 0) return [];
  if (count === 1) return [{ x: width / 2, y: height * 0.45 }];
  const radiusX = width * 0.42;
  const radiusY = height * 0.55;
  const centerY = height * 0.95;
  const span = Math.min(Math.PI * 0.9, Math.PI * 0.35 + (count - 2) * 0.22);
  const start = Math.PI / 2 + span / 2;
  return Array.from({ length: count }, (_, i) => {
    const angle = start - (span * i) / (count - 1);
    return { x: width / 2 + radiusX * Math.cos(angle), y: centerY - radiusY * Math.sin(angle) };
  });
}

/** The audience member as a participant, once they have spoken (v1.20.2). */
export const USER_SPEAKER_ID = "user";

/** Whether the audience member (the user) has spoken at least once. */
export function userHasSpoken(messages: { role: string }[]): boolean {
  return messages.some((m) => m.role === "user");
}

/** Participants who spoke (gladiateurs, and the audience member once they did), first appearance order. */
export function spokenParticipants(messages: { role: string; speakerId: string; speakerName: string }[]): { id: string; name: string }[] {
  const seen = new Map<string, string>();
  for (const m of messages) {
    if ((m.role === "GladIAteur" || m.role === "user") && !seen.has(m.speakerId)) seen.set(m.speakerId, m.speakerName);
  }
  return [...seen].map(([id, name]) => ({ id, name }));
}

export type ShortcutAction = "pauseResume" | "intervene" | "nextSpeaker" | "previousTurn" | "nextTurn" | "mute" | "presentation" | "sounds" | "help" | "exitPresentation";

/** Minimal view of a keyboard event (testable without the DOM). */
export interface ShortcutKey {
  key: string;
  /** Tag name of the focused element ("INPUT", "TEXTAREA", …) */
  targetTag?: string;
  editable?: boolean;
  ctrlKey?: boolean;
  metaKey?: boolean;
  altKey?: boolean;
}

/** Map a key press to an arena action; nothing while typing or with modifiers. */
export function shortcutAction(k: ShortcutKey): ShortcutAction | null {
  if (k.ctrlKey || k.metaKey || k.altKey) return null;
  if (k.editable || (k.targetTag && ["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(k.targetTag))) return null;
  switch (k.key) {
    case " ":
      return "pauseResume";
    case "i":
    case "I":
      return "intervene";
    case "ArrowLeft":
      return "previousTurn";
    case "ArrowRight":
      return "nextTurn";
    case "m":
    case "M":
      return "mute";
    case "p":
    case "P":
      return "presentation";
    case "s":
    case "S":
      return "sounds";
    case "n":
    case "N":
      return "nextSpeaker";
    case "?":
      return "help";
    case "Escape":
      return "exitPresentation";
    default:
      return null;
  }
}

/** The turn divider to scroll to, from the one currently in view. */
export function neighbourTurn(current: number, direction: "previousTurn" | "nextTurn", maxTurn: number): number | null {
  const next = direction === "previousTurn" ? current - 1 : current + 1;
  return next >= 1 && next <= maxTurn ? next : null;
}
