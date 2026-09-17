import { describe, expect, it } from "vitest";
import { cleanForSpeech, oceanFromPrompt, pickVoice, prosodyFromOcean, SpeechEngine, splitSentences } from "./speech";

describe("splitSentences", () => {
  it("cuts on Latin and CJK terminators and keeps the unfinished tail", () => {
    expect(splitSentences("Bonjour. Ça va ? Oui… et")).toEqual({ sentences: ["Bonjour.", "Ça va ?", "Oui…"], rest: "et" });
    expect(splitSentences("你好。再见！还有")).toEqual({ sentences: ["你好。", "再见！"], rest: "还有" });
    expect(splitSentences("Version 1.16 est sortie")).toEqual({ sentences: [], rest: "Version 1.16 est sortie" });
    expect(splitSentences("Fin.")).toEqual({ sentences: ["Fin."], rest: "" });
  });

  it("cleanForSpeech drops formulas and markup", () => {
    expect(cleanForSpeech("Soit $x^2$ et $$\\int f$$ **fort** `code`  ok")).toBe("Soit et fort code ok");
  });
});

describe("voices and prosody", () => {
  const voices = [
    { name: "Amélie", lang: "fr-FR" },
    { name: "Thomas", lang: "fr-FR" },
    { name: "Samantha", lang: "en-US", default: true },
  ];

  it("picks a stable voice of the language per speaker, the default otherwise", () => {
    const a = pickVoice(voices, "fr", "Le Scientifique");
    expect(a?.lang).toBe("fr-FR");
    expect(pickVoice(voices, "fr", "Le Scientifique")).toBe(a);
    expect(pickVoice(voices, "zh", "Quelqu'un")?.name).toBe("Samantha");
    expect(pickVoice([], "fr", "x")).toBeNull();
  });

  it("derives rate and pitch from extraversion and neuroticism", () => {
    expect(prosodyFromOcean(null)).toEqual({ rate: 1, pitch: 1 });
    expect(prosodyFromOcean([5, 5, 1, 5, 1])).toEqual({ rate: 0.9, pitch: 0.9 });
    expect(prosodyFromOcean([5, 5, 10, 5, 10])).toEqual({ rate: 1.15, pitch: 1.2 });
    expect(oceanFromPrompt("<persona/> O=8 C=3 E=9 A=2 N=6")).toEqual([8, 3, 9, 2, 6]);
    expect(oceanFromPrompt("no matrix")).toBeNull();
  });
});

describe("SpeechEngine", () => {
  class FakeSynth {
    spoken: string[] = [];
    getVoices() { return []; }
    speak(u: { text: string }) { this.spoken.push(u.text); }
    cancel() { this.spoken.push("<cancel>"); }
    pause() {}
    resume() {}
  }
  const fake = () => new FakeSynth() as unknown as SpeechSynthesis & FakeSynth;

  it("queues completed sentences from the stream and follows the speaker", () => {
    (globalThis as { SpeechSynthesisUtterance?: unknown }).SpeechSynthesisUtterance = class { text: string; constructor(t: string) { this.text = t; } };
    const synth = fake();
    const engine = new SpeechEngine(synth);
    engine.feed("g1", "Ignoré tant que désactivé.");
    expect(synth.spoken).toEqual([]);
    engine.enabled = true;
    engine.feed("g1", "Première phrase. Deux");
    engine.feed("g1", "ième phrase ! La sui");
    // The first sentence is being spoken, the second waits
    expect(synth.spoken).toEqual(["Première phrase."]);
    expect(engine.queued).toBe(1);
    engine.flush("g1");
    expect(engine.queued).toBe(2);
    // Another speaker starts (follow mode): g1's backlog is dropped
    engine.feed("g2", "Réponse nette.");
    expect(engine.queued).toBe(1);
    engine.mode = "full";
    engine.feed("g1", "Encore une.");
    expect(engine.queued).toBe(2);
    engine.stop();
    expect(engine.queued).toBe(0);
    expect(synth.spoken[synth.spoken.length - 1]).toBe("<cancel>");
  });

  it("degrades quietly without speech synthesis", () => {
    const engine = new SpeechEngine(null);
    expect(engine.available).toBe(false);
    engine.enabled = true;
    engine.feed("g1", "Rien ne casse.");
    engine.say("g1", "Rien.");
    expect(engine.queued).toBe(0);
  });
});
