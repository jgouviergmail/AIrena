// Procedural sounds of the arena (v1.18): everything is synthesised with the
// Web Audio API — no asset to ship. The context is created lazily and resumed
// on the first user gesture (autoplay policy); without Web Audio the engine
// stays silent.

export type SoundCue = "gong" | "murmur" | "applause" | "whistle";

/** Duration of each cue (s). */
export const SOUND_DURATION: Record<SoundCue, number> = { gong: 1.6, murmur: 1.2, applause: 1.4, whistle: 0.35 };

/** Minimal shape of an audio context (testable). */
export interface AudioContextLike {
  currentTime: number;
  state: string;
  destination: AudioNode;
  resume(): Promise<void>;
  createGain(): GainNode;
  createOscillator(): OscillatorNode;
  createBufferSource(): AudioBufferSourceNode;
  createBuffer(channels: number, length: number, sampleRate: number): AudioBuffer;
  createBiquadFilter(): BiquadFilterNode;
  sampleRate: number;
}

type ContextFactory = () => AudioContextLike | null;

const defaultFactory: ContextFactory = () => {
  if (typeof window === "undefined") return null;
  const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  return Ctor ? (new Ctor() as unknown as AudioContextLike) : null;
};

export class SoundEngine {
  private ctx: AudioContextLike | null = null;
  enabled = false;
  volume = 0.5;
  /** Cues played (for tests). */
  played: SoundCue[] = [];

  constructor(private readonly factory: ContextFactory = defaultFactory) {}

  /** Create / resume the context — call from a user gesture (autoplay policy). */
  async unlock(): Promise<void> {
    this.ctx ??= this.factory();
    if (this.ctx && this.ctx.state === "suspended") {
      try {
        await this.ctx.resume();
      } catch { /* stays suspended until the next gesture */ }
    }
  }

  get available(): boolean {
    return this.ctx !== null || this.factory() !== null;
  }

  play(cue: SoundCue) {
    if (!this.enabled || this.volume <= 0) return;
    this.ctx ??= this.factory();
    const ctx = this.ctx;
    if (!ctx || ctx.state === "suspended") return;
    this.played.push(cue);
    const master = ctx.createGain();
    master.gain.value = this.volume;
    master.connect(ctx.destination);
    switch (cue) {
      case "gong":
        this.gong(ctx, master);
        break;
      case "murmur":
        this.noise(ctx, master, SOUND_DURATION.murmur, 600, 0.25);
        break;
      case "applause":
        this.applause(ctx, master);
        break;
      case "whistle":
        this.whistle(ctx, master);
        break;
    }
  }

  /** A low sine with a long decay and a metallic partial. */
  private gong(ctx: AudioContextLike, out: AudioNode) {
    const now = ctx.currentTime;
    for (const [freq, level] of [[196, 0.6], [392, 0.25], [587, 0.1]] as const) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(level, now);
      gain.gain.exponentialRampToValueAtTime(0.001, now + SOUND_DURATION.gong);
      osc.connect(gain);
      gain.connect(out);
      osc.start(now);
      osc.stop(now + SOUND_DURATION.gong);
    }
  }

  /** Filtered noise with an envelope (murmur of the room). */
  private noise(ctx: AudioContextLike, out: AudioNode, seconds: number, cutoff: number, level: number) {
    const now = ctx.currentTime;
    const source = ctx.createBufferSource();
    const length = Math.floor(ctx.sampleRate * seconds);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = Math.random() * 2 - 1;
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = cutoff;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.001, now);
    gain.gain.linearRampToValueAtTime(level, now + seconds * 0.3);
    gain.gain.exponentialRampToValueAtTime(0.001, now + seconds);
    source.connect(filter);
    filter.connect(gain);
    gain.connect(out);
    source.start(now);
    source.stop(now + seconds);
  }

  /** Bursts of bright noise, like hands clapping. */
  private applause(ctx: AudioContextLike, out: AudioNode) {
    const now = ctx.currentTime;
    const bursts = 14;
    for (let i = 0; i < bursts; i++) {
      const start = now + (i * SOUND_DURATION.applause) / bursts + Math.random() * 0.03;
      const source = ctx.createBufferSource();
      const length = Math.floor(ctx.sampleRate * 0.06);
      const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let j = 0; j < length; j++) data[j] = (Math.random() * 2 - 1) * (1 - j / length);
      source.buffer = buffer;
      const filter = ctx.createBiquadFilter();
      filter.type = "highpass";
      filter.frequency.value = 1500;
      const gain = ctx.createGain();
      gain.gain.value = 0.35;
      source.connect(filter);
      filter.connect(gain);
      gain.connect(out);
      source.start(start);
    }
  }

  /** A short square-wave whistle (the moderator's sanction). */
  private whistle(ctx: AudioContextLike, out: AudioNode) {
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "square";
    osc.frequency.setValueAtTime(2200, now);
    osc.frequency.linearRampToValueAtTime(2600, now + SOUND_DURATION.whistle);
    gain.gain.setValueAtTime(0.15, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + SOUND_DURATION.whistle);
    osc.connect(gain);
    gain.connect(out);
    osc.start(now);
    osc.stop(now + SOUND_DURATION.whistle);
  }
}
