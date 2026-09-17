import type { ArenaReducer } from "../types";

/**
 * External knowledge fetched for the active speaker (web, Wikipedia, RAG) and
 * the engine's memory of positions. Counts accumulate globally and in a
 * pending slot that the next completed message adopts (see messagesReducer).
 */
export const knowledgeReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "positionsUpdated":
      return {
        positions: event.data.positions.map((p) => ({
          participantName: p.participantName,
          stance: p.stance,
          initialStance: p.initialStance ?? null,
          shift: p.shift ?? null,
          wouldChangeIf: p.wouldChangeIf ?? null,
        })),
      };
    case "webSearchPerformed":
      return {
        webSearchCount: s.webSearchCount + event.data.queries.length,
        _pendingSearchCount: s._pendingSearchCount + event.data.queries.length,
        activityStatus: { type: "webSearch", speakerName: event.data.speakerName },
      };
    case "wikiSearchPerformed":
      return {
        wikiSearchCount: s.wikiSearchCount + event.data.queries.length,
        _pendingWikiCount: s._pendingWikiCount + event.data.queries.length,
        _pendingWikiUrls: [...s._pendingWikiUrls, ...event.data.articleUrls],
        activityStatus: { type: "wikiSearch", speakerName: event.data.speakerName },
      };
    case "ragContextInjected":
      return {
        ragChunkCount: s.ragChunkCount + event.data.chunks.length,
        _pendingRagCount: s._pendingRagCount + event.data.chunks.length,
        _pendingRagChunks: [...s._pendingRagChunks, ...event.data.chunks],
        activityStatus: {
          type: event.data.cached ? "ragCacheHit" : "ragInjection",
          speakerName: event.data.speakerName,
        },
      };
    default:
      return null;
  }
};
