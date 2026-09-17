import { describe, expect, it } from "vitest";
import { effectiveTuning, parseTuningOverrides, serialiseTuningOverrides, sliderStep, TUNING_KEYS, type Tuning } from "./tuning";

const defaults: Tuning = {
  oceanGainMin: 0.6, oceanGainMax: 1.4, emotionTempSpan: 0.15, emotionTempMin: 0.3, emotionTempMax: 1.2, emotionLenMin: 0.8, emotionLenMax: 1.2,
  relationshipDecayPerTurn: 0.85, relationshipAllyScore: 2, relationshipRivalScore: 2, relationshipTenseScore: 2,
  sceneEventBaseProbability: 0.15, sceneEventStagnationBoost: 0.35, coalitionProbability: 0.2,
};

describe("advanced tuning", () => {
  it("keeps only known numeric knobs and serialises the differences", () => {
    expect(parseTuningOverrides("{}")).toEqual({});
    expect(parseTuningOverrides("nope")).toEqual({});
    expect(parseTuningOverrides(JSON.stringify({ coalitionProbability: 0.5, unknown: 1, emotionTempSpan: "x", oceanGainMax: NaN }))).toEqual({ coalitionProbability: 0.5 });
    const values = effectiveTuning(defaults, { coalitionProbability: 0.5 });
    expect(values.coalitionProbability).toBe(0.5);
    expect(values.oceanGainMin).toBe(0.6);
    expect(serialiseTuningOverrides(values, defaults)).toBe(JSON.stringify({ coalitionProbability: 0.5 }));
    expect(serialiseTuningOverrides(defaults, defaults)).toBe("{}");
    expect(TUNING_KEYS).toHaveLength(14);
  });

  it("derives a clean slider step from the bounds", () => {
    expect(sliderStep({ min: 0, max: 1 })).toBe(0.01);
    expect(sliderStep({ min: 0.5, max: 10 })).toBe(0.1);
    expect(sliderStep({ min: 1, max: 1 })).toBe(0.01);
  });
});
