import { describe, expect, it } from "vitest";
import { emptyArbitrePersona, emptyGladiateurPersona, parsePersona } from "./persona-parser";
import { serializePersona } from "./persona-serializer";
import type { PersonaData } from "./persona-types";

const gladiateur = (): PersonaData => {
  const p = emptyGladiateurPersona();
  p.identity = { nameTitle: "Le Sceptique, épistémologue", quote: "Douter d'abord.", biography: "Ancien chercheur en logique." };
  p.psychology = { ocean: { O: 8, C: 6, E: 3, A: 4, N: 7 }, posture: "PARENT_CRITIQUE", bias: "Biais de confirmation", blindSpot: "Sous-estime l'intuition" };
  p.voice = { register: "soutenu", syntax: "phrases courtes", tics: "« Voyons cela »", argumentation: "par l'absurde" };
  if (p.type === "gladiateur") {
    p.dynamics = { values: "rigueur", triggers: "approximations", underPressure: "devient sec", confident: "ironise", disengaged: "se tait" };
  }
  return p;
};

describe("persona parse ↔ serialize", () => {
  it.each(["fr", "en", "zh"] as const)("round-trips a gladiateur persona in %s", (lang) => {
    const original = gladiateur();
    const xml = serializePersona(original, lang);
    expect(xml).toContain("<persona>");
    const parsed = parsePersona(xml, "gladiateur");
    expect(parsed.success).toBe(true);
    if (!parsed.success) return;
    expect(parsed.data.identity.nameTitle).toBe(original.identity.nameTitle);
    expect(parsed.data.psychology.ocean).toEqual(original.psychology.ocean);
    expect(parsed.data.psychology.posture).toBe("PARENT_CRITIQUE");
    expect(parsed.data.voice.tics).toBe(original.voice.tics);
    if (parsed.data.type === "gladiateur" && original.type === "gladiateur") {
      expect(parsed.data.dynamics.underPressure).toBe(original.dynamics.underPressure);
    }
  });

  it("round-trips an arbitre persona with its moderation block", () => {
    const original = emptyArbitrePersona();
    original.identity.nameTitle = "La Modératrice";
    if (original.type === "arbitre") {
      original.moderation = { style: "ferme", redirection: "recentre", whenStagnates: "relance", whenDominates: "coupe" };
      original.dynamics = { underPressure: "reste calme", enthusiastic: "s'anime" };
    }
    const parsed = parsePersona(serializePersona(original, "fr"), "arbitre");
    expect(parsed.success).toBe(true);
    if (!parsed.success || parsed.data.type !== "arbitre") throw new Error("expected arbitre");
    expect(parsed.data.moderation.whenDominates).toBe("coupe");
    expect(parsed.data.dynamics.enthusiastic).toBe("s'anime");
  });

  it("rejects prompts that are not persona XML", () => {
    const r = parsePersona("Tu es un assistant utile.", "gladiateur");
    expect(r.success).toBe(false);
    if (!r.success) expect(r.raw).toContain("assistant");
  });
});
