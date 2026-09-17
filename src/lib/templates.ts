// Discussion templates (v1.20): a reusable setup saved by the user or seeded
// by the backend. The backend stores `configJson` as an opaque JSON object;
// this module is the single place that knows its shape.
import type {
  DiscussionFeatures,
  DiscussionMode,
  DiscussionTemplate,
  DocumentFormat,
  DocumentInjectionMode,
  GladIAteurConfig,
  IArbitreConfig,
  PredefinedProfile,
} from "./types";
import { DEFAULT_DISCUSSION_FEATURES, DEFAULT_LLM_PARAMS } from "./types";
import { DISCUSSION_MODES } from "./modes";

/** UI hint for the template name input (the backend enforces `TEMPLATE_NAME_MAX_CHARS`). */
export const TEMPLATE_NAME_MAX_CHARS = 80;

const DOCUMENT_FORMATS: readonly DocumentFormat[] = ["none", "txt", "md", "csv"];
const INJECTION_MODES: readonly DocumentInjectionMode[] = ["rag", "fullInjection"];

const optionalNumber = (v: unknown): number | undefined => (typeof v === "number" && Number.isFinite(v) ? v : undefined);
const optionalBoolean = (v: unknown): boolean | undefined => (typeof v === "boolean" ? v : undefined);
const optionalString = (v: unknown): string | undefined => (typeof v === "string" && v.trim() ? v : undefined);
const oneOf = <T extends string>(v: unknown, allowed: readonly T[]): T | undefined => (allowed.includes(v as T) ? (v as T) : undefined);

/** One gladiateur of a template: a catalogue profile plus its optional role and model. */
export interface TemplateGladiateur {
  profileId: string;
  modeRole?: string;
  model?: string;
}

/** What a template captures of the wizard (everything optional but the topic and mode). */
export interface TemplateConfig {
  topic: string;
  discussionLanguage: string;
  discussionMode: DiscussionMode;
  maxTurns: number | null;
  arbitreProfileId: string | null;
  gladiateurs: TemplateGladiateur[];
  features: DiscussionFeatures;
  documentFormat?: DocumentFormat;
  documentInjectionMode?: DocumentInjectionMode;
  argumentMapEnabled?: boolean;
  webSearchPool?: number;
  wikiSearchPool?: number;
}

/** The wizard state a template reads from and writes to. */
export interface TemplateSetupState {
  topic: string;
  discussionLanguage: string;
  discussionMode: DiscussionMode;
  maxTurns: number | null;
  arbitre: IArbitreConfig;
  gladiateurs: GladIAteurConfig[];
  features: DiscussionFeatures;
  documentFormat: DocumentFormat;
  documentInjectionMode: DocumentInjectionMode;
  argumentMapEnabled: boolean;
  webSearchPool: number;
  wikiSearchPool: number;
}

/** Resolve a profile's display name and prompt (translated builtin ones, raw custom ones). */
export type ProfileText = (profile: PredefinedProfile) => { name: string; systemPrompt: string };

/** Capture the current wizard as a template configuration (custom personas without a profile are skipped). */
export function templateFromSetup(state: TemplateSetupState, arbitreProfiles: PredefinedProfile[], profileText: ProfileText): TemplateConfig {
  const arbitre = arbitreProfiles.find((p) => profileText(p).name === state.arbitre.name) ?? null;
  return {
    topic: state.topic,
    discussionLanguage: state.discussionLanguage,
    discussionMode: state.discussionMode,
    maxTurns: state.maxTurns,
    arbitreProfileId: arbitre?.id ?? null,
    gladiateurs: state.gladiateurs
      .filter((g) => g.sourceProfileId)
      .map((g) => ({ profileId: g.sourceProfileId as string, ...(g.modeRole ? { modeRole: g.modeRole } : {}), ...(g.model ? { model: g.model } : {}) })),
    features: { ...state.features },
    documentFormat: state.documentFormat,
    documentInjectionMode: state.documentInjectionMode,
    argumentMapEnabled: state.argumentMapEnabled,
    webSearchPool: state.webSearchPool,
    wikiSearchPool: state.wikiSearchPool,
  };
}

/** Parse a stored configuration; null when it is not a usable template (the
 * stored JSON is data, never trusted: unknown modes are refused, wrong-typed
 * options are dropped). */
export function parseTemplateConfig(json: string): TemplateConfig | null {
  try {
    const parsed: unknown = JSON.parse(json);
    if (!parsed || typeof parsed !== "object") return null;
    const c = parsed as Record<string, unknown>;
    const mode = oneOf(c.discussionMode, DISCUSSION_MODES);
    if (typeof c.topic !== "string" || !mode) return null;
    const gladiateurs: TemplateGladiateur[] = Array.isArray(c.gladiateurs)
      ? c.gladiateurs
          .filter((g): g is Record<string, unknown> => !!g && typeof g === "object" && typeof (g as Record<string, unknown>).profileId === "string")
          .map((g) => ({ profileId: g.profileId as string, ...(optionalString(g.modeRole) ? { modeRole: g.modeRole as string } : {}), ...(optionalString(g.model) ? { model: g.model as string } : {}) }))
      : [];
    const features = c.features && typeof c.features === "object" ? (c.features as Partial<DiscussionFeatures>) : {};
    return {
      topic: c.topic,
      discussionLanguage: optionalString(c.discussionLanguage) ?? "fr",
      discussionMode: mode,
      maxTurns: optionalNumber(c.maxTurns) ?? null,
      arbitreProfileId: optionalString(c.arbitreProfileId) ?? null,
      gladiateurs,
      features: { ...DEFAULT_DISCUSSION_FEATURES, ...features },
      documentFormat: oneOf(c.documentFormat, DOCUMENT_FORMATS),
      documentInjectionMode: oneOf(c.documentInjectionMode, INJECTION_MODES),
      argumentMapEnabled: optionalBoolean(c.argumentMapEnabled),
      webSearchPool: optionalNumber(c.webSearchPool),
      wikiSearchPool: optionalNumber(c.wikiSearchPool),
    };
  } catch {
    return null;
  }
}

export interface AppliedTemplate {
  /** Wizard fields to set */
  patch: Partial<Omit<TemplateSetupState, "arbitre">>;
  /** Moderator persona to apply (null when the template names none or it is missing) */
  arbitre: { name: string; systemPrompt: string } | null;
  /** Profile ids the catalogue no longer has (skipped) */
  missing: string[];
}

/** Turn a template into wizard state: profiles resolved by id, missing ones reported (S40). */
export function applyTemplate(
  config: TemplateConfig,
  profiles: PredefinedProfile[],
  arbitreProfiles: PredefinedProfile[],
  profileText: ProfileText,
  newId: () => string,
): AppliedTemplate {
  const missing: string[] = [];
  const gladiateurs: GladIAteurConfig[] = [];
  config.gladiateurs.forEach((g) => {
    const profile = profiles.find((p) => p.id === g.profileId);
    if (!profile) {
      missing.push(g.profileId);
      return;
    }
    const text = profileText(profile);
    gladiateurs.push({
      id: newId(),
      name: text.name,
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: text.systemPrompt,
      llmParams: { ...DEFAULT_LLM_PARAMS },
      sourceProfileId: profile.id,
      initialEmotions: profile.initialEmotions,
      ...(g.modeRole ? { modeRole: g.modeRole } : {}),
      ...(g.model ? { model: g.model } : {}),
    });
  });
  const patch: Partial<Omit<TemplateSetupState, "arbitre">> = {
    topic: config.topic,
    discussionLanguage: config.discussionLanguage,
    discussionMode: config.discussionMode,
    maxTurns: config.maxTurns,
    gladiateurs,
    features: { ...config.features },
  };
  if (config.documentFormat !== undefined) patch.documentFormat = config.documentFormat;
  if (config.documentInjectionMode !== undefined) patch.documentInjectionMode = config.documentInjectionMode;
  if (config.argumentMapEnabled !== undefined) patch.argumentMapEnabled = config.argumentMapEnabled;
  if (config.webSearchPool !== undefined) patch.webSearchPool = config.webSearchPool;
  if (config.wikiSearchPool !== undefined) patch.wikiSearchPool = config.wikiSearchPool;
  let arbitre: AppliedTemplate["arbitre"] = null;
  if (config.arbitreProfileId) {
    const profile = arbitreProfiles.find((p) => p.id === config.arbitreProfileId);
    if (profile) {
      const text = profileText(profile);
      arbitre = { name: text.name, systemPrompt: text.systemPrompt };
    } else {
      missing.push(config.arbitreProfileId);
    }
  }
  return { patch, arbitre, missing };
}

/** A user template ready to be saved (the backend fills `createdAt`). */
export function newTemplate(name: string, config: TemplateConfig): DiscussionTemplate {
  return { id: `tpl-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, name: name.trim(), configJson: JSON.stringify(config), builtin: false, createdAt: "" };
}
