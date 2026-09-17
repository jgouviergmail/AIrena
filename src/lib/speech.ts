// Voice of the arena (v1.18): the browser's speech synthesis reads the
// interventions sentence by sentence as they stream. Pure parts (sentence
// splitting, voice choice, prosody from OCEAN) are tested in node; the engine
// wraps `window.speechSynthesis` and degrades quietly when it is missing.

export type TtsMode = "follow" | "full";

/** Sentence terminators: Latin ones need a following space or the end, CJK ones stand alone. */
const SENTENCE_END = /[.!?…](?=\s|$)|[。！？]/;
/** KaTeX blocks and inline formulas are not read aloud. */
const FORMULA = /\$\$[\s\S]*?\$\$|\$[^$\n]+\$/g;
/** Markdown residue the voice should not pronounce. */
const MARKUP = /[*_#`>]+/g;

/** Text worth reading: formulas and markup stripped, whitespace folded. */
export function cleanForSpeech(text: string): string {
  return text.replace(FORMULA, " ").replace(MARKUP, "").replace(/\s+/g, " ").trim();
}

/**
 * Split a text into complete sentences, keeping the unfinished tail. CJK
 * sentences end on 。！？, Latin ones on . ! ? … followed by a space or the end.
 */
export function splitSentences(text: string): { sentences: string[]; rest: string } {
  const sentences: string[] = [];
  let rest = text;
  for (;;) {
    const m = SENTENCE_END.exec(rest);
    if (!m) break;
    const end = m.index + m[0].length;
    const sentence = rest.slice(0, end).trim();
    if (sentence) sentences.push(sentence);
    rest = rest.slice(end).trimStart();
  }
  return { sentences, rest };
}

/** Voice-like shape, so the choice is testable without the browser. */
export interface VoiceLike {
  name: string;
  lang: string;
  default?: boolean;
}

const LANG_PREFIX: Record<string, string> = { fr: "fr", en: "en", zh: "zh" };

/**
 * The voice for a speaker: among the voices of the discussion language, one
 * chosen by a hash of the speaker's name (stable, distinct per persona); the
 * default voice when the language has none; `null` when there is no voice at all.
 */
export function pickVoice<V extends VoiceLike>(voices: V[], lang: string, speakerName: string): V | null {
  if (voices.length === 0) return null;
  const prefix = LANG_PREFIX[lang] ?? lang;
  const matching = voices.filter((v) => v.lang.toLowerCase().startsWith(prefix));
  const pool = matching.length ? matching : voices;
  if (!matching.length) return voices.find((v) => v.default) ?? voices[0];
  let hash = 0;
  for (let i = 0; i < speakerName.length; i++) hash = (hash * 31 + speakerName.charCodeAt(i)) >>> 0;
  return pool[hash % pool.length];
}

/** Rate and pitch of a persona from its OCEAN matrix (extraversion → pace, neuroticism → pitch swing). */
export function prosodyFromOcean(ocean: [number, number, number, number, number] | null): { rate: number; pitch: number } {
  if (!ocean) return { rate: 1, pitch: 1 };
  const [, , e, , n] = ocean;
  const rate = 0.9 + ((Math.min(10, Math.max(1, e)) - 1) / 9) * 0.25;
  const pitch = 0.9 + ((Math.min(10, Math.max(1, n)) - 1) / 9) * 0.3;
  return { rate: Math.round(rate * 100) / 100, pitch: Math.round(pitch * 100) / 100 };
}

/** OCEAN values from a persona prompt ("O=x C=x E=x A=x N=x"), or null. */
export function oceanFromPrompt(systemPrompt: string): [number, number, number, number, number] | null {
  const m = /O=(\d+)\s*C=(\d+)\s*E=(\d+)\s*A=(\d+)\s*N=(\d+)/.exec(systemPrompt);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3]), Number(m[4]), Number(m[5])] : null;
}

interface Utterance {
  speakerId: string;
  text: string;
}

export interface SpeakerVoiceProfile {
  name: string;
  ocean: [number, number, number, number, number] | null;
}

/**
 * Queue of sentences read one after the other. `feed` accumulates streamed
 * chunks per speaker and enqueues each completed sentence; `flush` reads the
 * unfinished tail when a message completes. In `follow` mode a change of
 * speaker drops what was still queued for the previous one.
 */
export class SpeechEngine {
  private queue: Utterance[] = [];
  private pending = new Map<string, string>();
  private speaking = false;
  private currentSpeaker: string | null = null;
  /** Paused by the user (the queue keeps growing; `resume` picks it up). */
  paused = false;
  enabled = false;
  mode: TtsMode = "follow";
  volume = 1;
  lang = "fr";
  profiles = new Map<string, SpeakerVoiceProfile>();

  constructor(private readonly synth: SpeechSynthesis | null = typeof window !== "undefined" ? window.speechSynthesis ?? null : null) {}

  /** The platform has speech synthesis. */
  get available(): boolean {
    return this.synth !== null;
  }

  /** Streamed chunk of a speaker: completed sentences are queued at once. */
  feed(speakerId: string, chunk: string) {
    if (!this.enabled || !this.synth) return;
    const buffer = (this.pending.get(speakerId) ?? "") + chunk;
    const { sentences, rest } = splitSentences(buffer);
    this.pending.set(speakerId, rest);
    for (const s of sentences) this.enqueue(speakerId, s);
  }

  /** The message ended: read the unfinished tail. */
  flush(speakerId: string) {
    const rest = this.pending.get(speakerId) ?? "";
    this.pending.delete(speakerId);
    if (this.enabled && this.synth && rest.trim()) this.enqueue(speakerId, rest);
  }

  /** Read a whole text (the synthesis, a stage line…). */
  say(speakerId: string, text: string) {
    if (!this.enabled || !this.synth) return;
    const { sentences, rest } = splitSentences(text);
    for (const s of [...sentences, rest]) if (s.trim()) this.enqueue(speakerId, s);
  }

  pause() {
    this.paused = true;
    this.synth?.pause();
  }

  resume() {
    this.paused = false;
    this.synth?.resume();
    this.next();
  }

  /** Drop everything and stop the current utterance. */
  stop() {
    this.queue = [];
    this.pending.clear();
    this.speaking = false;
    this.paused = false;
    this.currentSpeaker = null;
    this.synth?.cancel();
  }

  /** Utterances waiting (for tests and the status). */
  get queued(): number {
    return this.queue.length;
  }

  private enqueue(speakerId: string, raw: string) {
    const text = cleanForSpeech(raw);
    if (!text) return;
    if (this.mode === "follow" && this.currentSpeaker && this.currentSpeaker !== speakerId) {
      // Stay live: what the previous speaker had not said yet is dropped
      this.queue = this.queue.filter((u) => u.speakerId === speakerId);
    }
    this.queue.push({ speakerId, text });
    this.next();
  }

  private next() {
    if (this.speaking || this.paused || !this.synth) return;
    const item = this.queue.shift();
    if (!item) return;
    this.speaking = true;
    this.currentSpeaker = item.speakerId;
    const utterance = new SpeechSynthesisUtterance(item.text);
    const profile = this.profiles.get(item.speakerId);
    const voice = pickVoice(this.synth.getVoices(), this.lang, profile?.name ?? item.speakerId);
    if (voice) utterance.voice = voice;
    utterance.lang = voice?.lang ?? this.lang;
    const { rate, pitch } = prosodyFromOcean(profile?.ocean ?? null);
    utterance.rate = rate;
    utterance.pitch = pitch;
    utterance.volume = this.volume;
    const done = () => {
      this.speaking = false;
      this.next();
    };
    utterance.onend = done;
    utterance.onerror = done;
    this.synth.speak(utterance);
  }
}
