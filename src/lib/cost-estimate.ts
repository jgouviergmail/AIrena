import type { ModelPriceInfo } from "./types";

/**
 * Rough per-turn cost model for a cloud provider (order of magnitude, not a quote).
 *
 * Calls per turn with native reasoning (no separate "thought" call):
 * per gladiateur → intervention + reaction + moderation; per turn → memory + emotions.
 * Utility prompts are much shorter than the intervention prompt and produce
 * short JSON answers.
 */
export const UTILITY_PROMPT_RATIO = 0.25;
export const UTILITY_OUTPUT_TOKENS = 200;
export const TURN_LEVEL_PROMPT_RATIO = 0.5;
export const TURN_LEVEL_OUTPUT_TOKENS = 400;
/** Share of prompt tokens served from the provider's prefix cache once system prompts repeat. */
export const DEFAULT_CACHE_HIT_RATIO = 0.5;
const MILLION = 1_000_000;

export interface TurnCostInput {
  price: ModelPriceInfo;
  /** Prompt tokens of one intervention call (the used context budget). */
  promptTokens: number;
  /** Generated tokens of one intervention (≈ numPredict / 2). */
  outputTokens: number;
  nGladiateurs: number;
  /** Off-peak prices are this fraction of peak prices. */
  offpeakFactor: number;
  cacheHitRatio?: number;
}

export interface TurnCostEstimate {
  peakUsd: number;
  offpeakUsd: number;
  promptTokensPerTurn: number;
  outputTokensPerTurn: number;
  callsPerTurn: number;
}

export function estimateTurnCost(input: TurnCostInput): TurnCostEstimate {
  const n = Math.max(1, input.nGladiateurs);
  const hit = Math.min(1, Math.max(0, input.cacheHitRatio ?? DEFAULT_CACHE_HIT_RATIO));
  const p = Math.max(0, input.promptTokens);
  const o = Math.max(0, input.outputTokens);

  const promptTokensPerTurn = Math.round(n * (p + 2 * p * UTILITY_PROMPT_RATIO) + 2 * p * TURN_LEVEL_PROMPT_RATIO);
  const outputTokensPerTurn = Math.round(n * (o + 2 * UTILITY_OUTPUT_TOKENS) + 2 * TURN_LEVEL_OUTPUT_TOKENS);
  const callsPerTurn = n * 3 + 2;

  const inputPricePerToken =
    (hit * input.price.inputCacheHit + (1 - hit) * input.price.inputCacheMiss) / MILLION;
  const outputPricePerToken = input.price.output / MILLION;
  const peakUsd = promptTokensPerTurn * inputPricePerToken + outputTokensPerTurn * outputPricePerToken;

  return {
    peakUsd,
    offpeakUsd: peakUsd * input.offpeakFactor,
    promptTokensPerTurn,
    outputTokensPerTurn,
    callsPerTurn,
  };
}

/** Compact token counter ("950", "1.2k", "12k", "2.5M"). */
export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}k`;
  if (n >= 1_000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

/** Compact USD formatting for small amounts ("$0.0042", "$1.20"). */
export function formatUsd(value: number | null | undefined, digits?: number): string {
  if (value === null || value === undefined || Number.isNaN(value)) return "—";
  const d = digits ?? (value < 0.01 ? 4 : value < 1 ? 3 : 2);
  return `$${value.toFixed(d)}`;
}
