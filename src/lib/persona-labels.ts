import type { PostureValue } from "./persona-types";

// ---------------------------------------------------------------------------
// Supported serialization languages
// ---------------------------------------------------------------------------

export type Lang = "fr" | "en" | "zh";

// ---------------------------------------------------------------------------
// Field labels — single source of truth for parser + serializer
// Each entry maps a field key → { fr, en, zh } label strings.
// ---------------------------------------------------------------------------

export const FIELD_LABELS: Record<string, Record<Lang, string>> = {
  ocean:         { fr: "OCEAN:",                          en: "OCEAN:",                            zh: "OCEAN:" },
  posture:       { fr: "Posture:",                        en: "Posture:",                          zh: "姿态:" },
  bias:          { fr: "Biais:",                          en: "Bias:",                             zh: "偏见:" },
  blindSpot:     { fr: "Angle mort:",                     en: "Blind spot:",                       zh: "盲点:" },
  register:      { fr: "Registre:",                       en: "Register:",                         zh: "语域:" },
  syntax:        { fr: "Syntaxe:",                        en: "Syntax:",                           zh: "句法:" },
  tics:          { fr: "Tics:",                           en: "Tics:",                             zh: "口头禅:" },
  argumentation: { fr: "Argumentation:",                  en: "Argumentation:",                    zh: "论证:" },
  values:        { fr: "Valeurs:",                        en: "Values:",                           zh: "价值观:" },
  triggers:      { fr: "Déclencheurs:",                   en: "Triggers:",                         zh: "触发点:" },
  underPressure: { fr: "Sous pression:",                  en: "Under pressure:",                   zh: "承压时:" },
  confident:     { fr: "En confiance:",                   en: "Confident:",                        zh: "自信时:" },
  disengaged:    { fr: "Désengagé:",                      en: "Disengaged:",                       zh: "无兴趣时:" },
  style:         { fr: "Style:",                          en: "Style:",                            zh: "风格:" },
  redirection:   { fr: "Recadrage:",                      en: "Redirection:",                      zh: "纠偏:" },
  whenStagnates: { fr: "Quand le débat stagne:",          en: "When debate stagnates:",            zh: "辩论停滞时:" },
  whenDominates: { fr: "Quand un participant domine:",    en: "When a participant dominates:",     zh: "参与者主导时:" },
  enthusiastic:  { fr: "Enthousiaste:",                   en: "Enthusiastic:",                     zh: "热情时:" },
};

// ---------------------------------------------------------------------------
// Posture translations — single source of truth
// ---------------------------------------------------------------------------

export const POSTURE_TRANSLATIONS: Record<PostureValue, Record<Lang, string>> = {
  ADULTE:            { fr: "ADULTE",            en: "ADULT",            zh: "成人" },
  ENFANT_LIBRE:      { fr: "ENFANT_LIBRE",      en: "FREE_CHILD",      zh: "自由儿童" },
  "ENFANT_ADAPTÉ":   { fr: "ENFANT_ADAPTÉ",     en: "ADAPTED_CHILD",   zh: "顺应儿童" },
  PARENT_CRITIQUE:   { fr: "PARENT_CRITIQUE",   en: "CRITICAL_PARENT", zh: "批判父母" },
  PARENT_NOURRICIER: { fr: "PARENT_NOURRICIER", en: "NURTURING_PARENT", zh: "养育父母" },
};

// ---------------------------------------------------------------------------
// Derived maps (used by parser and serializer)
// ---------------------------------------------------------------------------

/** All language variants for a field label — used by the parser to detect labels in any language. */
export function fieldLabelVariants(fieldName: string): string[] {
  const entry = FIELD_LABELS[fieldName];
  if (!entry) return [];
  return [entry.fr, entry.en, entry.zh];
}

/** Reverse posture map: any translation → canonical PostureValue. Used by the parser. */
export const POSTURE_REVERSE: Record<string, PostureValue> = (() => {
  const map: Record<string, PostureValue> = {};
  for (const [canonical, translations] of Object.entries(POSTURE_TRANSLATIONS)) {
    for (const label of Object.values(translations)) {
      map[label] = canonical as PostureValue;
    }
  }
  // Chinese synonym: 适应儿童 is an alternate translation for ENFANT_ADAPTÉ
  map["适应儿童"] = "ENFANT_ADAPTÉ";
  return map;
})();
