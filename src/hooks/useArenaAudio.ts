import { useEffect } from "react";
import { SpeechEngine, oceanFromPrompt } from "@/lib/speech";
import { SoundEngine } from "@/lib/sounds";
import { useArenaStore, type AudioSink } from "@/stores/useArenaStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useSetupStore } from "@/stores/useSetupStore";
import type { ArenaEvent } from "@/lib/types";

/** One voice and one sound engine for the whole app (the arena mounts and unmounts). */
export const speech = new SpeechEngine();
export const sounds = new SoundEngine();

/** What each engine event triggers on the sound side. */
function cueFor(event: ArenaEvent): Parameters<SoundEngine["play"]>[0] | null {
  switch (event.type) {
    case "turnStarted":
      return "gong";
    case "banIssued":
      return "whistle";
    case "sceneEventTriggered":
      return "murmur";
    case "reactionEmitted":
      // The room applauds its own approvals
      return event.data.reaction.fromSpeakerId === "user" && ["like", "insightful", "laugh"].includes(event.data.reaction.reactionType) ? "applause" : null;
    default:
      return null;
  }
}

/**
 * Settings → engines, app-wide (mounted in the shell): switching the voice off
 * anywhere — the summary page included — silences it at once.
 */
export function useAudioSettingsSync() {
  const settings = useSettingsStore((s) => s.settings);
  useEffect(() => {
    speech.enabled = settings.ttsEnabled;
    speech.mode = settings.ttsMode;
    speech.volume = settings.ttsVolume;
    sounds.enabled = settings.soundEnabled;
    sounds.volume = settings.soundVolume;
    if (!settings.ttsEnabled) speech.stop();
  }, [settings.ttsEnabled, settings.ttsMode, settings.ttsVolume, settings.soundEnabled, settings.soundVolume]);
}

/**
 * Voice and sounds of the arena (v1.18): feeds the speech engine with the
 * streamed text, reads the synthesis with the moderator's voice, plays the
 * cues, and unlocks the audio context on the first gesture (autoplay policy).
 * Leaving the arena silences the voice, unless the discussion just ended (the
 * synthesis keeps being read on the summary page).
 */
export function useArenaAudio() {
  const language = useSetupStore((s) => s.discussionLanguage);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const registerAudioSink = useArenaStore((s) => s.registerAudioSink);
  const unregisterAudioSink = useArenaStore((s) => s.unregisterAudioSink);

  // Cast → voices
  useEffect(() => {
    speech.lang = language;
    speech.profiles = new Map([
      [arbitre.id, { name: arbitre.name, ocean: oceanFromPrompt(arbitre.systemPrompt) }],
      ...gladiateurs.map((g) => [g.id, { name: g.name, ocean: oceanFromPrompt(g.systemPrompt) }] as const),
    ]);
  }, [language, arbitre, gladiateurs]);

  // Engine events → speech and cues
  useEffect(() => {
    const sink: AudioSink = {
      onChunk: (speakerId, chunk) => speech.feed(speakerId, chunk),
      onEvent: (event) => {
        const cue = cueFor(event);
        if (cue) sounds.play(cue);
        switch (event.type) {
          case "messageComplete":
            if (event.data.message.kind === undefined || event.data.message.kind === "normal") speech.flush(event.data.message.speakerId);
            break;
          case "synthesisComplete":
            speech.say(arbitre.id, event.data.summary);
            break;
          case "pauseConfirmed":
            speech.pause();
            break;
          case "resumeConfirmed":
            speech.resume();
            break;
          case "discussionStarted":
            speech.stop();
            break;
        }
      },
    };
    registerAudioSink(sink);
    return () => unregisterAudioSink();
  }, [registerAudioSink, unregisterAudioSink, arbitre.id]);

  // Autoplay policy: the context runs once the user has touched the page
  useEffect(() => {
    void sounds.unlock();
    const unlock = () => void sounds.unlock();
    window.addEventListener("pointerdown", unlock, { once: true });
    window.addEventListener("keydown", unlock, { once: true });
    return () => {
      window.removeEventListener("pointerdown", unlock);
      window.removeEventListener("keydown", unlock);
    };
  }, []);

  // Leaving the arena silences the voice — except right after the end (the synthesis is being read)
  useEffect(() => () => { if (useArenaStore.getState().status !== "ended") speech.stop(); }, []);
}
