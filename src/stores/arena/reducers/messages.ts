import type { Message, Reaction } from "@/lib/types";
import type { ArenaReducer } from "../types";

/** An engine-confirmed audience reaction replaces its optimistic (pending) twin; others append. */
function confirmReaction(list: Message["reactions"], reaction: Reaction): Message["reactions"] {
  if (reaction.fromSpeakerId === "user") {
    const idx = list.findIndex((r) => r.pending && r.fromSpeakerId === "user" && r.reactionType === reaction.reactionType);
    if (idx >= 0) return list.map((r, i) => (i === idx ? reaction : r));
  }
  return [...list, reaction];
}

/**
 * Messages and reactions. A completed message adopts the knowledge (web /
 * wiki / RAG) collected for the speaker since `speakerActive`, so the feed
 * badges land on the right bubble.
 */
export const messagesReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "messageComplete": {
      if (!event.data.message?.id) {
        console.error("Received messageComplete with invalid message", event.data);
        return null;
      }
      const msg = {
        ...event.data.message,
        reactions: Array.isArray(event.data.message.reactions) ? event.data.message.reactions : [],
      };
      return {
        messages: [...s.messages, msg],
        webSearchesPerMessage: s._pendingSearchCount > 0
          ? { ...s.webSearchesPerMessage, [msg.id]: s._pendingSearchCount }
          : s.webSearchesPerMessage,
        wikiSearchesPerMessage: s._pendingWikiCount > 0
          ? { ...s.wikiSearchesPerMessage, [msg.id]: s._pendingWikiCount }
          : s.wikiSearchesPerMessage,
        wikiArticleUrlsPerMessage: s._pendingWikiUrls.length > 0
          ? { ...s.wikiArticleUrlsPerMessage, [msg.id]: [...s._pendingWikiUrls] }
          : s.wikiArticleUrlsPerMessage,
        ragChunksPerMessage: s._pendingRagCount > 0
          ? { ...s.ragChunksPerMessage, [msg.id]: s._pendingRagCount }
          : s.ragChunksPerMessage,
        ragChunkDetailsPerMessage: s._pendingRagChunks.length > 0
          ? { ...s.ragChunkDetailsPerMessage, [msg.id]: [...s._pendingRagChunks] }
          : s.ragChunkDetailsPerMessage,
        _pendingSearchCount: 0,
        _pendingWikiCount: 0,
        _pendingWikiUrls: [],
        _pendingRagCount: 0,
        _pendingRagChunks: [],
      };
    }
    case "reactionEmitted": {
      const targetId = event.data.messageId;
      const reaction = event.data.reaction;
      return {
        messages: s.messages.map((m) => (m.id === targetId ? { ...m, reactions: confirmReaction(m.reactions ?? [], reaction) } : m)),
        activityStatus: s.activityStatus?.type === "reacting"
          ? s.activityStatus
          : { type: "reacting", speakerName: reaction.fromSpeakerName },
      };
    }
    default:
      return null;
  }
};
