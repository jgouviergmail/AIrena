import { describe, expect, it } from "vitest";
import { applyTemplate, newTemplate, parseTemplateConfig, templateFromSetup, type TemplateSetupState } from "./templates";
import { DEFAULT_DISCUSSION_FEATURES, DEFAULT_LLM_PARAMS, type PredefinedProfile } from "./types";

const profile = (id: string, name: string, type = "gladiateur"): PredefinedProfile => ({
  id, name, personality: "", systemPrompt: `<persona>${name}</persona>`, isBuiltin: true, profileType: type, category: "experts",
});
const profiles = [profile("scientist", "Le Scientifique"), profile("lawyer", "L'Avocat")];
const arbitres = [profile("arb-strict", "Le Juge Strict", "arbitre")];
const text = (p: PredefinedProfile) => ({ name: p.name, systemPrompt: p.systemPrompt });
let n = 0;
const newId = () => `g-${++n}`;

describe("templates", () => {
  it("applies a template: profiles resolved by id, roles and models kept, missing ones reported (S40)", () => {
    const config = parseTemplateConfig(JSON.stringify({
      topic: "Procès", discussionMode: "trial", maxTurns: 6, arbitreProfileId: "arb-strict",
      gladiateurs: [{ profileId: "lawyer", modeRole: "prosecutor" }, { profileId: "ghost" }, { profileId: "scientist", model: "mistral" }],
      features: { hiddenAgenda: false },
    }))!;
    expect(config.features).toEqual({ ...DEFAULT_DISCUSSION_FEATURES, hiddenAgenda: false });
    const applied = applyTemplate(config, profiles, arbitres, text, newId);
    expect(applied.missing).toEqual(["ghost"]);
    expect(applied.arbitre).toEqual({ name: "Le Juge Strict", systemPrompt: "<persona>Le Juge Strict</persona>" });
    expect(applied.patch.topic).toBe("Procès");
    expect(applied.patch.discussionMode).toBe("trial");
    expect(applied.patch.gladiateurs?.map((g) => [g.name, g.interventionNumber, g.modeRole, g.model, g.sourceProfileId])).toEqual([
      ["L'Avocat", 1, "prosecutor", undefined, "lawyer"],
      ["Le Scientifique", 2, undefined, "mistral", "scientist"],
    ]);
    expect(applied.patch.gladiateurs?.[0].llmParams).toEqual(DEFAULT_LLM_PARAMS);
    // Optional fields left untouched when the template does not carry them
    expect("documentFormat" in applied.patch).toBe(false);
    // A missing moderator is reported too
    const noArb = applyTemplate({ ...config, arbitreProfileId: "arb-ghost" }, profiles, arbitres, text, newId);
    expect(noArb.arbitre).toBeNull();
    expect(noArb.missing).toEqual(["ghost", "arb-ghost"]);
  });

  it("rejects unusable configurations", () => {
    expect(parseTemplateConfig("nope")).toBeNull();
    expect(parseTemplateConfig("[1]")).toBeNull();
    expect(parseTemplateConfig(JSON.stringify({ discussionMode: "debate" }))).toBeNull();
    expect(parseTemplateConfig(JSON.stringify({ topic: "t", discussionMode: "debate", gladiateurs: [null, { x: 1 }] }))?.gladiateurs).toEqual([]);
    // An unknown mode (a template from another version) is not usable; wrong-typed options are dropped
    expect(parseTemplateConfig(JSON.stringify({ topic: "t", discussionMode: "karaoke" }))).toBeNull();
    const loose = parseTemplateConfig(JSON.stringify({
      topic: "t", discussionMode: "debate", maxTurns: "6", documentFormat: "pdf", documentInjectionMode: "rag", argumentMapEnabled: "yes", webSearchPool: "3", wikiSearchPool: 2,
      gladiateurs: [{ profileId: "scientist", modeRole: 4, model: 7 }],
    }))!;
    expect(loose.maxTurns).toBeNull();
    expect(loose.documentFormat).toBeUndefined();
    expect(loose.documentInjectionMode).toBe("rag");
    expect(loose.argumentMapEnabled).toBeUndefined();
    expect(loose.webSearchPool).toBeUndefined();
    expect(loose.wikiSearchPool).toBe(2);
    expect(loose.gladiateurs).toEqual([{ profileId: "scientist" }]);
  });

  it("captures the wizard as a template, skipping custom personas", () => {
    const state: TemplateSetupState = {
      topic: "Sujet", discussionLanguage: "en", discussionMode: "debate", maxTurns: null,
      arbitre: { id: "a", name: "Le Juge Strict", systemPrompt: "x", turnDistribution: "sequential", llmParams: DEFAULT_LLM_PARAMS },
      gladiateurs: [
        { id: "1", name: "Le Scientifique", interventionNumber: 1, systemPrompt: "", llmParams: DEFAULT_LLM_PARAMS, sourceProfileId: "scientist", modeRole: "for" },
        { id: "2", name: "Perso", interventionNumber: 2, systemPrompt: "", llmParams: DEFAULT_LLM_PARAMS },
      ],
      features: DEFAULT_DISCUSSION_FEATURES, documentFormat: "none", documentInjectionMode: "rag", argumentMapEnabled: true, webSearchPool: 2, wikiSearchPool: 0,
    };
    const config = templateFromSetup(state, arbitres, text);
    expect(config.arbitreProfileId).toBe("arb-strict");
    expect(config.gladiateurs).toEqual([{ profileId: "scientist", modeRole: "for" }]);
    expect(config.webSearchPool).toBe(2);
    const tpl = newTemplate("  Mon modèle ", config);
    expect(tpl.name).toBe("Mon modèle");
    expect(tpl.builtin).toBe(false);
    expect(parseTemplateConfig(tpl.configJson)?.topic).toBe("Sujet");
  });
});
