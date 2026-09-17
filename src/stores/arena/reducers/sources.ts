import type { ArenaReducer } from "../types";
import { fromRagChunk, fromWebResult, fromWikiArticle, isCited } from "@/lib/sources";

/**
 * References injected into the active speaker's prompt. They wait in
 * `_pendingSources` until the speaker's message completes, then get attached
 * to it with the "probably cited" heuristic.
 */
export const sourcesReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "speakerActive":
      return { _pendingSources: [] };
    case "webSearchPerformed": {
      const items = (event.data.results ?? []).map((r) => fromWebResult(r, s.currentTurn, event.data.speakerId, event.data.speakerName));
      return items.length ? { _pendingSources: [...s._pendingSources, ...items] } : null;
    }
    case "wikiSearchPerformed": {
      const items = (event.data.articles ?? []).map((a) => fromWikiArticle(a, s.currentTurn, event.data.speakerId, event.data.speakerName));
      return items.length ? { _pendingSources: [...s._pendingSources, ...items] } : null;
    }
    case "ragContextInjected": {
      const items = event.data.chunks.map((c) => fromRagChunk(c, s.currentTurn, event.data.speakerId, event.data.speakerName));
      return items.length ? { _pendingSources: [...s._pendingSources, ...items] } : null;
    }
    case "messageComplete": {
      const msg = event.data.message;
      if (!msg?.id || s._pendingSources.length === 0) return null;
      const attached = s._pendingSources.map((p) => ({ ...p, messageId: msg.id, cited: isCited(msg.content, p) }));
      return { sources: [...s.sources, ...attached], _pendingSources: [] };
    }
    default:
      return null;
  }
};
