import type { DocumentFormat } from "@/lib/types";
import type { ArenaReducer } from "../types";

/** Co-constructed document, argument map, usage, budget alerts and the unveiled agendas. */
export const artifactsReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "documentUpdated":
      return {
        previousDocumentContent: s.documentContent || null,
        documentContent: event.data.content,
        documentFormat: event.data.format as DocumentFormat,
        documentLastEditor: event.data.speakerName,
      };
    case "argumentMapUpdated":
      return {
        argumentMapMarkdown: event.data.markdown,
        argumentMapMarkdownBySpeaker: event.data.markdownBySpeaker,
        argumentMapThesesCount: event.data.thesesCount,
        argumentMapArgumentsCount: event.data.argumentsCount,
        argumentMap: event.data.map,
        argumentMapNewNodeIds: event.data.newNodeIds,
        argumentMapDroppedCount: event.data.droppedCount,
        argumentMapDepth: event.data.depth ?? null,
        activityStatus: { type: "argumentMap" },
      };
    case "llmUsageUpdated":
      return { llmUsage: event.data };
    case "budgetAlert":
      return { budgetAlert: event.data };
    case "agendaRevealed":
      return { agendas: event.data.agendas };
    case "personaRecapReady":
      return { recaps: [...s.recaps, event.data.recap] };
    default:
      return null;
  }
};
