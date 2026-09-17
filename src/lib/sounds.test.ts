import { describe, expect, it } from "vitest";
import { SoundEngine, type AudioContextLike } from "./sounds";

/** A context that records what was built, enough for the engine's graphs. */
function fakeContext(state = "running"): AudioContextLike & { nodes: string[] } {
  const nodes: string[] = [];
  const param = () => ({ value: 0, setValueAtTime() {}, linearRampToValueAtTime() {}, exponentialRampToValueAtTime() {} });
  const node = (kind: string) => {
    nodes.push(kind);
    return {
      connect() {},
      start() {},
      stop() {},
      gain: param(),
      frequency: param(),
      type: "sine",
      buffer: null,
    } as unknown as GainNode & OscillatorNode & AudioBufferSourceNode & BiquadFilterNode;
  };
  return {
    nodes,
    currentTime: 0,
    state,
    sampleRate: 8000,
    destination: {} as AudioNode,
    resume: async () => {},
    createGain: () => node("gain"),
    createOscillator: () => node("osc"),
    createBufferSource: () => node("source"),
    createBiquadFilter: () => node("filter"),
    createBuffer: (_c: number, length: number) => ({ getChannelData: () => new Float32Array(length) }) as unknown as AudioBuffer,
  };
}

describe("SoundEngine", () => {
  it("plays nothing while disabled, muted or suspended; synthesises every cue otherwise", () => {
    const ctx = fakeContext();
    const engine = new SoundEngine(() => ctx);
    engine.play("gong");
    expect(engine.played).toEqual([]);
    engine.enabled = true;
    engine.volume = 0;
    engine.play("gong");
    expect(engine.played).toEqual([]);
    engine.volume = 0.5;
    for (const cue of ["gong", "murmur", "applause", "whistle"] as const) engine.play(cue);
    expect(engine.played).toEqual(["gong", "murmur", "applause", "whistle"]);
    // three gong partials + the whistle
    expect(ctx.nodes.filter((n) => n === "osc").length).toBe(4);
    expect(ctx.nodes).toContain("filter");

    const suspended = new SoundEngine(() => fakeContext("suspended"));
    suspended.enabled = true;
    suspended.play("gong");
    expect(suspended.played).toEqual([]);
  });

  it("stays silent without Web Audio", () => {
    const engine = new SoundEngine(() => null);
    engine.enabled = true;
    expect(engine.available).toBe(false);
    engine.play("applause");
    expect(engine.played).toEqual([]);
  });
});
