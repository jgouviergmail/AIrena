// Local casting analysis (v1.19): probable affinities between the selected
// gladiateurs, from the OCEAN scores of their personas — no model call.
import { parseOceanScores } from "./persona-parser";
import type { OceanScores } from "./persona-types";

/** Thresholds of the compatibility heuristic (OCEAN axes are scored 1–10). */
export const COMPATIBILITY = {
  /** Mean A/E/O distance at or under which two similar, agreeable profiles are probable allies */
  alliesMaxDistance: 2,
  /** Summed agreeableness two allies need (both at least average) */
  alliesMinAgreeableness: 10,
  /** Mean A/E/O distance from which two profiles will probably clash */
  frictionMinDistance: 4.5,
  /** Agreeableness at or under which a profile is frontal: two of them clash whatever their distance */
  frontalMaxAgreeableness: 3,
  /** Mean pairwise distance under which a cast is low-contrast, and from which it is high-contrast */
  contrastLow: 2,
  contrastHigh: 4,
} as const;

export type CompatibilityKind = "allies" | "friction" | "neutral";

export interface CastMember {
  id: string;
  name: string;
  systemPrompt: string;
}

export interface CompatibilityPair {
  a: CastMember;
  b: CastMember;
  kind: CompatibilityKind;
  /** Mean absolute difference over A, E and O (0–9) */
  distance: number;
}

export type ContrastLevel = "low" | "medium" | "high";

export interface CastAnalysis {
  pairs: CompatibilityPair[];
  /** Mean pairwise distance (null below two scored members) */
  contrast: number | null;
  contrastLevel: ContrastLevel | null;
  /** Members whose persona carries no OCEAN scores (left out of the pairs) */
  unscored: CastMember[];
}

/** Mean absolute difference over the three social axes (agreeableness, extraversion, openness). */
export function socialDistance(x: OceanScores, y: OceanScores): number {
  return (Math.abs(x.A - y.A) + Math.abs(x.E - y.E) + Math.abs(x.O - y.O)) / 3;
}

/** Probable relationship of two profiles: alike and agreeable → allies; far apart or both frontal → friction. */
export function classifyPair(x: OceanScores, y: OceanScores): CompatibilityKind {
  const d = socialDistance(x, y);
  const bothFrontal = x.A <= COMPATIBILITY.frontalMaxAgreeableness && y.A <= COMPATIBILITY.frontalMaxAgreeableness;
  if (bothFrontal || d >= COMPATIBILITY.frictionMinDistance) return "friction";
  if (d <= COMPATIBILITY.alliesMaxDistance && x.A + y.A >= COMPATIBILITY.alliesMinAgreeableness) return "allies";
  return "neutral";
}

export function contrastLevel(contrast: number): ContrastLevel {
  if (contrast < COMPATIBILITY.contrastLow) return "low";
  if (contrast >= COMPATIBILITY.contrastHigh) return "high";
  return "medium";
}

/** Every pair of scored members, plus the overall contrast of the cast. */
export function analyseCast(members: CastMember[]): CastAnalysis {
  const scored: { member: CastMember; ocean: OceanScores }[] = [];
  const unscored: CastMember[] = [];
  for (const member of members) {
    const ocean = parseOceanScores(member.systemPrompt);
    if (ocean) scored.push({ member, ocean });
    else unscored.push(member);
  }
  const pairs: CompatibilityPair[] = [];
  for (let i = 0; i < scored.length; i++) {
    for (let j = i + 1; j < scored.length; j++) {
      const distance = socialDistance(scored[i].ocean, scored[j].ocean);
      pairs.push({ a: scored[i].member, b: scored[j].member, kind: classifyPair(scored[i].ocean, scored[j].ocean), distance });
    }
  }
  const contrast = pairs.length > 0 ? pairs.reduce((sum, p) => sum + p.distance, 0) / pairs.length : null;
  return { pairs, contrast, contrastLevel: contrast === null ? null : contrastLevel(contrast), unscored };
}
