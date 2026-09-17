import type { ArenaReducer } from "../types";

let thresholdSeq = 0;

/** Emotions, their history, threshold crossings and the backstage (directives, intentions). */
export const emotionsReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "emotionUpdated": {
      const emotions = new Map(s.emotions).set(event.data.speakerId, event.data.emotions);
      const moodSummary = event.data.moodSummary
        ? new Map(s.moodSummary).set(event.data.speakerId, event.data.moodSummary)
        : s.moodSummary;
      return {
        emotions,
        moodSummary,
        activityStatus: s.activityStatus?.type === "emotions" ? s.activityStatus : { type: "emotions" },
      };
    }
    case "emotionHistoryUpdate":
      return { emotionHistory: new Map(s.emotionHistory).set(event.data.speakerId, event.data.history) };
    case "roomMoodUpdated":
      return { roomMood: { avg: event.data.avg, label: event.data.label } };
    case "emotionalThresholdCrossed":
      return { lastThresholdCrossed: { ...event.data, seq: ++thresholdSeq } };
    case "directiveGenerated": {
      const directives = new Map(s.directives).set(event.data.speakerId, {
        speechAct: event.data.speechAct,
        emotionBehavior: event.data.emotionBehavior ?? null,
        relationshipSummary: event.data.relationshipSummary,
        focusSpeaker: event.data.focusSpeaker ?? null,
        reasoningLevel: event.data.reasoningLevel ?? "off",
      });
      return { directives };
    }
    case "intentionGenerated": {
      const { speakerId, target, goal, angle, concession, question } = event.data;
      const intentions = new Map(s.intentions).set(speakerId, {
        target: target ?? null,
        goal,
        angle: angle ?? "",
        concession: concession ?? null,
        question: question ?? null,
      });
      const granted = concession || goal === "concede";
      return granted ? { intentions, concessions: { ...s.concessions, [speakerId]: (s.concessions[speakerId] ?? 0) + 1 } } : { intentions };
    }
    default:
      return null;
  }
};
