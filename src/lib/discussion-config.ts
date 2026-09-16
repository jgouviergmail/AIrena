import type { DiscussionConfig } from "./types";

/**
 * The global context setting applies to every speaker: Ollama's context window
 * (KV cache) or DeepSeek's context budget. Per-speaker `numCtx` is not exposed
 * in the wizard, so the config is normalised before starting (spread copies —
 * the setup store is never mutated).
 */
export function applyGlobalContext(config: DiscussionConfig, numCtx: number): DiscussionConfig {
  return {
    ...config,
    arbitre: { ...config.arbitre, llmParams: { ...config.arbitre.llmParams, numCtx } },
    gladiateurs: config.gladiateurs.map((g) => ({ ...g, llmParams: { ...g.llmParams, numCtx } })),
  };
}
