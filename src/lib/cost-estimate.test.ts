import { describe, expect, it } from "vitest";
import { DEFAULT_CACHE_HIT_RATIO, estimateTurnCost, formatUsd } from "./cost-estimate";
import type { ModelPriceInfo } from "./types";

// deepseek-flash price list (USD / 1M tokens, peak)
const FLASH: ModelPriceInfo = { model: "deepseek-flash", inputCacheHit: 0.006, inputCacheMiss: 0.3, output: 1.2 };

describe("estimateTurnCost", () => {
  it("scales with the number of gladiateurs and halves off-peak", () => {
    const two = estimateTurnCost({ price: FLASH, promptTokens: 8000, outputTokens: 500, nGladiateurs: 2, offpeakFactor: 0.5 });
    const four = estimateTurnCost({ price: FLASH, promptTokens: 8000, outputTokens: 500, nGladiateurs: 4, offpeakFactor: 0.5 });
    expect(four.peakUsd).toBeGreaterThan(two.peakUsd);
    expect(four.callsPerTurn).toBe(4 * 3 + 2);
    expect(two.offpeakUsd).toBeCloseTo(two.peakUsd / 2, 10);
    // Order of magnitude: a few tenths of a cent per turn on flash
    expect(two.peakUsd).toBeGreaterThan(0.001);
    expect(two.peakUsd).toBeLessThan(0.05);
  });

  it("uses the cache-hit ratio on prompt tokens only", () => {
    // Output price zeroed to isolate the prompt side (utility calls always emit some output tokens)
    const inputOnly: ModelPriceInfo = { ...FLASH, output: 0 };
    const noCache = estimateTurnCost({ price: inputOnly, promptTokens: 10_000, outputTokens: 0, nGladiateurs: 1, offpeakFactor: 0.5, cacheHitRatio: 0 });
    const allCached = estimateTurnCost({ price: inputOnly, promptTokens: 10_000, outputTokens: 0, nGladiateurs: 1, offpeakFactor: 0.5, cacheHitRatio: 1 });
    expect(noCache.promptTokensPerTurn).toBe(allCached.promptTokensPerTurn);
    expect(allCached.peakUsd).toBeLessThan(noCache.peakUsd / 10);
    // Output tokens are never cached: with only output, the ratio changes nothing
    const outA = estimateTurnCost({ price: FLASH, promptTokens: 0, outputTokens: 1000, nGladiateurs: 1, offpeakFactor: 0.5, cacheHitRatio: 0 });
    const outB = estimateTurnCost({ price: FLASH, promptTokens: 0, outputTokens: 1000, nGladiateurs: 1, offpeakFactor: 0.5, cacheHitRatio: 1 });
    expect(outA.peakUsd).toBeCloseTo(outB.peakUsd, 12);
  });

  it("clamps degenerate inputs instead of producing NaN or negatives", () => {
    const r = estimateTurnCost({ price: FLASH, promptTokens: -5, outputTokens: -1, nGladiateurs: 0, offpeakFactor: 0.5, cacheHitRatio: 7 });
    expect(r.peakUsd).toBeGreaterThanOrEqual(0);
    expect(Number.isFinite(r.peakUsd)).toBe(true);
    expect(r.callsPerTurn).toBe(5); // 1 gladiateur minimum
    expect(DEFAULT_CACHE_HIT_RATIO).toBeGreaterThan(0);
  });
});

describe("formatUsd", () => {
  it("adapts precision to the magnitude and handles unknown values", () => {
    expect(formatUsd(0.0042)).toBe("$0.0042");
    expect(formatUsd(0.123)).toBe("$0.123");
    expect(formatUsd(12.3456)).toBe("$12.35");
    expect(formatUsd(null)).toBe("—");
    expect(formatUsd(undefined)).toBe("—");
    expect(formatUsd(Number.NaN)).toBe("—");
    expect(formatUsd(0.5, 1)).toBe("$0.5");
  });
});
