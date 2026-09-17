import type { TimelineEntry } from "@/lib/report";
import { STAGE_MAX_BURSTS } from "@/lib/stage";
import type { ArenaData, ArenaReducer, StageBanner } from "../types";

let nextEffectId = 0;

/** Append a notable moment anchored to the message the feed is at. */
function withMoment(s: ArenaData, entry: Omit<TimelineEntry, "messageIndex">): Partial<ArenaData> {
  return { timeline: [...s.timeline, { ...entry, messageIndex: s.messages.length }] };
}

/** A full-width banner (the component dismisses it after `BANNER_MS`). */
function banner(kind: StageBanner["kind"], text: string, speakerId?: string): StageBanner {
  return { id: `banner-${++nextEffectId}`, kind, text, speakerId };
}

/**
 * Staging of the discussion (v1.18): the act in progress, the scene event and
 * the coalition of the turn, the floating reactions, the banners and the
 * timeline of notable moments (acts, scene events, coalitions, bans, shifts,
 * user turns). The stage, the banners and the timeline bar read this slice;
 * the report persists the timeline.
 */
export const stageReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "turnStarted":
      return { sceneEvent: null, coalition: null, banner: banner("turn", String(event.data.turnNumber)) };
    case "actStarted":
      return {
        currentAct: { key: event.data.act, title: event.data.title, turn: event.data.turn },
        banner: banner("act", event.data.title),
        ...withMoment(s, { kind: "act", turn: event.data.turn, label: event.data.title }),
      };
    case "sceneEventTriggered":
      return {
        sceneEvent: { turn: event.data.turn, event: event.data.event, participants: event.data.participants },
        banner: banner("sceneEvent", event.data.event.kind),
        ...withMoment(s, { kind: "sceneEvent", turn: event.data.turn, label: event.data.event.kind }),
      };
    case "coalitionFormed":
      return {
        coalition: { a: event.data.a, b: event.data.b },
        ...withMoment(s, { kind: "coalition", turn: event.data.turn, label: `${event.data.aName} + ${event.data.bName}`, speakerId: event.data.a }),
      };
    case "banIssued":
      return {
        banner: banner("ban", event.data.bannedName, event.data.bannedId),
        ...withMoment(s, { kind: "ban", turn: s.currentTurn, label: event.data.bannedName, speakerId: event.data.bannedId }),
      };
    case "banLifted":
      return withMoment(s, { kind: "banLifted", turn: s.currentTurn, label: event.data.speakerName, speakerId: event.data.speakerId });
    case "relationshipShift":
      return withMoment(s, { kind: "relationshipShift", turn: s.currentTurn, label: `${event.data.from} → ${event.data.to}`, speakerId: event.data.a });
    case "userTurnReady":
      return withMoment(s, { kind: "userTurn", turn: s.currentTurn, label: "" });
    case "reactionEmitted": {
      // The target speaker is the author of the message reacted to
      const target = s.messages.find((m) => m.id === event.data.messageId)?.speakerId;
      if (!target) return null;
      const burst = { id: `burst-${++nextEffectId}`, from: event.data.reaction.fromSpeakerId, to: target, reactionType: event.data.reaction.reactionType };
      return { bursts: [...s.bursts, burst].slice(-STAGE_MAX_BURSTS) };
    }
    default:
      return null;
  }
};
