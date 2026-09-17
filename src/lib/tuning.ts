// Advanced tuning (v1.20): the user's overrides of the engine's dynamics,
// persisted as a JSON object of only the knobs that differ from the defaults.
// The backend (`engine::tuning::Tuning`) owns the defaults and the bounds.

export type TuningKey =
  | "oceanGainMin" | "oceanGainMax"
  | "emotionTempSpan" | "emotionTempMin" | "emotionTempMax" | "emotionLenMin" | "emotionLenMax"
  | "relationshipDecayPerTurn" | "relationshipAllyScore" | "relationshipRivalScore" | "relationshipTenseScore"
  | "sceneEventBaseProbability" | "sceneEventStagnationBoost" | "coalitionProbability";

export type Tuning = Record<TuningKey, number>;

export interface TuningBounds {
  min: number;
  max: number;
}

/** Defaults and bounds, as served by `get_tuning_info`. */
export interface TuningInfo {
  defaults: Tuning;
  bounds: Record<TuningKey, TuningBounds>;
}

/** Knobs in display order, grouped for the settings section. */
export const TUNING_GROUPS: { key: string; knobs: TuningKey[] }[] = [
  { key: "emotions", knobs: ["oceanGainMin", "oceanGainMax", "emotionTempSpan", "emotionTempMin", "emotionTempMax", "emotionLenMin", "emotionLenMax"] },
  { key: "relations", knobs: ["relationshipDecayPerTurn", "relationshipAllyScore", "relationshipRivalScore", "relationshipTenseScore"] },
  { key: "staging", knobs: ["sceneEventBaseProbability", "sceneEventStagnationBoost", "coalitionProbability"] },
];

export const TUNING_KEYS: TuningKey[] = TUNING_GROUPS.flatMap((g) => g.knobs);

/** Parse the persisted override: only known numeric knobs survive. */
export function parseTuningOverrides(json: string): Partial<Tuning> {
  try {
    const parsed: unknown = JSON.parse(json || "{}");
    if (!parsed || typeof parsed !== "object") return {};
    const out: Partial<Tuning> = {};
    for (const key of TUNING_KEYS) {
      const v = (parsed as Record<string, unknown>)[key];
      if (typeof v === "number" && Number.isFinite(v)) out[key] = v;
    }
    return out;
  } catch {
    return {};
  }
}

/** The effective values: defaults overridden by the persisted knobs. */
export function effectiveTuning(defaults: Tuning, overrides: Partial<Tuning>): Tuning {
  return { ...defaults, ...overrides };
}

/** Serialise only what differs from the defaults ("{}" when nothing does), so the backend keeps its constants. */
export function serialiseTuningOverrides(values: Tuning, defaults: Tuning): string {
  const out: Partial<Tuning> = {};
  for (const key of TUNING_KEYS) {
    if (values[key] !== defaults[key]) out[key] = values[key];
  }
  return JSON.stringify(out);
}

/** Slider step from the bounds: about 100 positions, rounded to a clean decimal. */
export function sliderStep(bounds: TuningBounds): number {
  const span = bounds.max - bounds.min;
  if (span <= 0) return 0.01;
  const raw = span / 100;
  const magnitude = Math.pow(10, Math.floor(Math.log10(raw)));
  return Number((Math.ceil(raw / magnitude) * magnitude).toPrecision(2));
}
