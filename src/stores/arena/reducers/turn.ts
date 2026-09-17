import type { ArenaReducer } from "../types";

/** Turn lifecycle: order, active speaker, bans, passes, user turn, pause. */
export const turnReducer: ArenaReducer = (s, event) => {
  switch (event.type) {
    case "turnStarted": {
      // Decrement ban counters (a ban issued this turn is not decremented yet)
      const bans = new Map(s.bans);
      for (const [id, info] of bans) {
        if (info.remaining <= 0) continue;
        bans.set(id, info.justIssued ? { ...info, justIssued: false } : { ...info, remaining: info.remaining - 1 });
      }
      return {
        currentTurn: event.data.turnNumber,
        speakerOrder: event.data.speakerOrder,
        interventionRequested: false,
        determiningOrder: false,
        bans,
        // A vote window left open (nobody voted) closes when the debate moves on
        voteRequest: null,
      };
    }
    case "rolesAssigned":
      return { roles: new Map(event.data.roles.map((r) => [r.speakerId, r])) };
    case "awaitingCue":
      return { awaitingCue: event.data };
    case "audienceVoteRequested":
      return { voteRequest: { phase: event.data.phase, timeoutSecs: event.data.timeoutSecs } };
    case "audienceVoteRecorded":
      return { voteRequest: null, votes: { ...s.votes, [event.data.phase]: event.data.choice } };
    case "outcomeReady":
      return { outcome: event.data.outcome, voteRequest: null };
    case "turnSkipped":
      return { determiningOrder: false };
    case "determiningOrder":
      return { determiningOrder: true, activityStatus: { type: "determining" } };
    case "speakerPassed":
      return { passedSpeakerIds: [...s.passedSpeakerIds, event.data.speakerId] };
    case "relationshipsUpdated":
      return { relationships: event.data.edges };
    case "speakerActive":
      // The speaker name for the activity label is resolved by the store (needs the setup store)
      return {
        activeSpeakerId: event.data.speakerId,
        _pendingSearchCount: 0,
        _pendingWikiCount: 0,
        _pendingWikiUrls: [],
        _pendingRagCount: 0,
        _pendingRagChunks: [],
      };
    case "banIssued": {
      const bans = new Map(s.bans);
      const prev = bans.get(event.data.bannedId);
      bans.set(event.data.bannedId, {
        remaining: event.data.duration,
        totalBans: (prev?.totalBans ?? 0) + 1,
        justIssued: true,
      });
      return { bans };
    }
    case "banLifted": {
      const prev = s.bans.get(event.data.speakerId);
      if (!prev) return null;
      const bans = new Map(s.bans).set(event.data.speakerId, { remaining: 0, totalBans: prev.totalBans, justIssued: false });
      return { bans };
    }
    case "userTurnReady":
      // A new user turn starts a new round of respond/pass decisions
      return { userTurnActive: true, interventionRequested: false, passedSpeakerIds: [] };
    case "userTurnTimeout":
      return { userTurnActive: false };
    case "turnTimings":
      return { timings: [...s.timings, event.data.timings] };
    case "diagnosticsReady":
      return { diagnostics: event.data.diagnostics };
    case "pauseConfirmed":
      return { status: "paused", activityStatus: null };
    case "resumeConfirmed":
      return { status: "running" };
    default:
      return null;
  }
};
