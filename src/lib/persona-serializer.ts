import type { PersonaData } from "./persona-types";
import { type Lang, FIELD_LABELS, POSTURE_TRANSLATIONS } from "./persona-labels";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Get a field label for a specific language, with fallback to French. */
function label(fieldName: string, lang: Lang): string {
  const entry = FIELD_LABELS[fieldName];
  if (!entry) return "";
  return entry[lang] ?? entry.fr;
}

/** Get the posture translation for a specific language. */
function postureLabel(posture: string, lang: Lang): string {
  const entry = POSTURE_TRANSLATIONS[posture as keyof typeof POSTURE_TRANSLATIONS];
  if (!entry) return posture;
  return entry[lang] ?? entry.fr;
}

// ---------------------------------------------------------------------------
// Serializer
// ---------------------------------------------------------------------------

export function serializePersona(data: PersonaData, lang: Lang): string {
  const { identity, psychology, voice } = data;
  const { ocean, posture } = psychology;

  // --- Identity ---
  const quoteLine = identity.quote ? `\n"${identity.quote}"` : "";
  const bioLine = identity.biography ? `\n${identity.biography}` : "";
  const identityBlock = `<identity>\n${identity.nameTitle}${quoteLine}${bioLine}\n</identity>`;

  // --- Psychology ---
  const oceanLine = `${label("ocean", lang)} O=${ocean.O} C=${ocean.C} E=${ocean.E} A=${ocean.A} N=${ocean.N}`;
  const postureLine = `${label("posture", lang)} ${postureLabel(posture, lang)}`;
  let psychLines = `${oceanLine}\n${postureLine}`;
  if (psychology.bias) {
    psychLines += `\n${label("bias", lang)} ${psychology.bias}`;
  }
  if (psychology.blindSpot) {
    psychLines += `\n${label("blindSpot", lang)} ${psychology.blindSpot}`;
  }
  const psychologyBlock = `<psychology>\n${psychLines}\n</psychology>`;

  // --- Voice ---
  let voiceLines = `${label("register", lang)} ${voice.register}`;
  voiceLines += `\n${label("syntax", lang)} ${voice.syntax}`;
  if (voice.tics) {
    voiceLines += `\n${label("tics", lang)} ${voice.tics}`;
  }
  if (voice.argumentation) {
    voiceLines += `\n${label("argumentation", lang)} ${voice.argumentation}`;
  }
  const voiceBlock = `<voice>\n${voiceLines}\n</voice>`;

  // --- Type-specific sections ---
  if (data.type === "arbitre") {
    const { moderation, dynamics } = data;

    let modLines = `${label("style", lang)} ${moderation.style}`;
    modLines += `\n${label("redirection", lang)} ${moderation.redirection}`;
    modLines += `\n${label("whenStagnates", lang)} ${moderation.whenStagnates}`;
    modLines += `\n${label("whenDominates", lang)} ${moderation.whenDominates}`;
    const moderationBlock = `<moderation>\n${modLines}\n</moderation>`;

    let dynLines = `${label("underPressure", lang)} ${dynamics.underPressure}`;
    dynLines += `\n${label("enthusiastic", lang)} ${dynamics.enthusiastic}`;
    const dynamicsBlock = `<dynamics>\n${dynLines}\n</dynamics>`;

    return `<persona>\n${identityBlock}\n${psychologyBlock}\n${voiceBlock}\n${moderationBlock}\n${dynamicsBlock}\n</persona>`;
  }

  // Gladiateur
  const { dynamics } = data;
  let dynLines = `${label("values", lang)} ${dynamics.values}`;
  dynLines += `\n${label("triggers", lang)} ${dynamics.triggers}`;
  dynLines += `\n${label("underPressure", lang)} ${dynamics.underPressure}`;
  dynLines += `\n${label("confident", lang)} ${dynamics.confident}`;
  dynLines += `\n${label("disengaged", lang)} ${dynamics.disengaged}`;
  const dynamicsBlock = `<dynamics>\n${dynLines}\n</dynamics>`;

  return `<persona>\n${identityBlock}\n${psychologyBlock}\n${voiceBlock}\n${dynamicsBlock}\n</persona>`;
}
