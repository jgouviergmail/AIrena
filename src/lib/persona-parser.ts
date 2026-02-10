import type {
  ParseResult,
  PersonaData,
  IdentityData,
  PsychologyData,
  VoiceData,
  DynamicsData,
  ModerationData,
  ArbitreDynamicsData,
  OceanScores,
  PostureValue,
} from "./persona-types";
import { fieldLabelVariants, POSTURE_REVERSE } from "./persona-labels";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function extractSection(xml: string, tag: string): string | null {
  const re = new RegExp(`<${tag}>\\s*([\\s\\S]*?)\\s*</${tag}>`, "i");
  const m = xml.match(re);
  return m ? m[1].trim() : null;
}

/**
 * Collect all known labels that could appear in a section so we can determine
 * where one field ends and the next begins.
 */
function allLabelsFlat(fieldNames: string[]): string[] {
  const out: string[] = [];
  for (const name of fieldNames) {
    out.push(...fieldLabelVariants(name));
  }
  return out;
}

/**
 * Extract text for a labelled field inside a section.
 * Tries all language variants for the label. Returns text from after the label
 * until the next known label or end of section.
 */
function extractField(
  sectionText: string,
  fieldName: string,
  siblingFields: string[],
): string {
  const variants = fieldLabelVariants(fieldName);
  if (variants.length === 0) return "";

  for (const label of variants) {
    const idx = sectionText.indexOf(label);
    if (idx === -1) continue;

    const afterLabel = idx + label.length;
    // Find end: earliest position of any sibling label after our position
    let end = sectionText.length;
    const allSiblings = allLabelsFlat(siblingFields);
    for (const sib of allSiblings) {
      const sibIdx = sectionText.indexOf(sib, afterLabel);
      if (sibIdx !== -1 && sibIdx < end) {
        end = sibIdx;
      }
    }
    return sectionText.substring(afterLabel, end).trim();
  }
  return "";
}

// ---------------------------------------------------------------------------
// Section parsers
// ---------------------------------------------------------------------------

function parseIdentity(raw: string): IdentityData {
  const lines = raw
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);

  const nameTitle = lines[0] ?? "";

  // Find the quote line — starts with " or \u201C or \u00AB
  const quoteLineIdx = lines.findIndex((l) =>
    /^[""«\u201C]/.test(l),
  );
  let quote = "";
  if (quoteLineIdx !== -1) {
    quote = lines[quoteLineIdx].replace(/^[""«\u201C]+|[""»\u201D]+$/g, "").trim();
  }

  // Biography = everything after the quote line
  const bioStart = quoteLineIdx !== -1 ? quoteLineIdx + 1 : 2;
  const biography = lines.slice(bioStart).join("\n").trim();

  return { nameTitle, quote, biography };
}

function parseOcean(text: string): OceanScores {
  const m = text.match(/O=(\d+)\s+C=(\d+)\s+E=(\d+)\s+A=(\d+)\s+N=(\d+)/);
  if (!m) return { O: 5, C: 5, E: 5, A: 5, N: 5 };
  return { O: +m[1], C: +m[2], E: +m[3], A: +m[4], N: +m[5] };
}

function parsePosture(text: string): PostureValue {
  const trimmed = text.trim();
  return POSTURE_REVERSE[trimmed] ?? "ADULTE";
}

const PSYCHOLOGY_FIELDS = ["posture", "bias", "blindSpot"];

function parsePsychology(raw: string): PsychologyData {
  const ocean = parseOcean(raw);

  const postureRaw = extractField(raw, "posture", PSYCHOLOGY_FIELDS);
  const posture = parsePosture(postureRaw);

  const bias = extractField(raw, "bias", PSYCHOLOGY_FIELDS);
  const blindSpot = extractField(raw, "blindSpot", PSYCHOLOGY_FIELDS);

  return { ocean, posture, bias, blindSpot };
}

const VOICE_FIELDS = ["register", "syntax", "tics", "argumentation"];

function parseVoice(raw: string): VoiceData {
  return {
    register: extractField(raw, "register", VOICE_FIELDS),
    syntax: extractField(raw, "syntax", VOICE_FIELDS),
    tics: extractField(raw, "tics", VOICE_FIELDS),
    argumentation: extractField(raw, "argumentation", VOICE_FIELDS),
  };
}

const DYNAMICS_FIELDS = [
  "values",
  "triggers",
  "underPressure",
  "confident",
  "disengaged",
];

function parseDynamics(raw: string): DynamicsData {
  return {
    values: extractField(raw, "values", DYNAMICS_FIELDS),
    triggers: extractField(raw, "triggers", DYNAMICS_FIELDS),
    underPressure: extractField(raw, "underPressure", DYNAMICS_FIELDS),
    confident: extractField(raw, "confident", DYNAMICS_FIELDS),
    disengaged: extractField(raw, "disengaged", DYNAMICS_FIELDS),
  };
}

const MODERATION_FIELDS = [
  "style",
  "redirection",
  "whenStagnates",
  "whenDominates",
];

function parseModeration(raw: string): ModerationData {
  return {
    style: extractField(raw, "style", MODERATION_FIELDS),
    redirection: extractField(raw, "redirection", MODERATION_FIELDS),
    whenStagnates: extractField(raw, "whenStagnates", MODERATION_FIELDS),
    whenDominates: extractField(raw, "whenDominates", MODERATION_FIELDS),
  };
}

const ARBITRE_DYNAMICS_FIELDS = ["underPressure", "enthusiastic"];

function parseArbitreDynamics(raw: string): ArbitreDynamicsData {
  return {
    underPressure: extractField(raw, "underPressure", ARBITRE_DYNAMICS_FIELDS),
    enthusiastic: extractField(raw, "enthusiastic", ARBITRE_DYNAMICS_FIELDS),
  };
}

// ---------------------------------------------------------------------------
// Main parser
// ---------------------------------------------------------------------------

export function parsePersona(
  systemPrompt: string,
  profileType: "gladiateur" | "arbitre",
): ParseResult {
  if (!systemPrompt.includes("<persona>")) {
    return { success: false, raw: systemPrompt };
  }

  const identityRaw = extractSection(systemPrompt, "identity");
  const psychologyRaw = extractSection(systemPrompt, "psychology");
  const voiceRaw = extractSection(systemPrompt, "voice");

  if (!identityRaw || !psychologyRaw || !voiceRaw) {
    return { success: false, raw: systemPrompt };
  }

  const identity = parseIdentity(identityRaw);
  const psychology = parsePsychology(psychologyRaw);
  const voice = parseVoice(voiceRaw);

  const hasModeration = systemPrompt.includes("<moderation>");
  const isArbitre = profileType === "arbitre" || hasModeration;

  if (isArbitre) {
    const moderationRaw = extractSection(systemPrompt, "moderation");
    const dynamicsRaw = extractSection(systemPrompt, "dynamics");
    const moderation = moderationRaw
      ? parseModeration(moderationRaw)
      : { style: "", redirection: "", whenStagnates: "", whenDominates: "" };
    const dynamics = dynamicsRaw
      ? parseArbitreDynamics(dynamicsRaw)
      : { underPressure: "", enthusiastic: "" };

    const data: PersonaData = {
      type: "arbitre",
      identity,
      psychology,
      voice,
      moderation,
      dynamics,
    };
    return { success: true, data };
  }

  const dynamicsRaw = extractSection(systemPrompt, "dynamics");
  const dynamics = dynamicsRaw
    ? parseDynamics(dynamicsRaw)
    : {
        values: "",
        triggers: "",
        underPressure: "",
        confident: "",
        disengaged: "",
      };

  const data: PersonaData = {
    type: "gladiateur",
    identity,
    psychology,
    voice,
    dynamics,
  };
  return { success: true, data };
}

// Default empty personas for new custom profiles
export function emptyGladiateurPersona(): PersonaData {
  return {
    type: "gladiateur",
    identity: { nameTitle: "", quote: "", biography: "" },
    psychology: {
      ocean: { O: 5, C: 5, E: 5, A: 5, N: 5 },
      posture: "ADULTE",
      bias: "",
      blindSpot: "",
    },
    voice: { register: "", syntax: "", tics: "", argumentation: "" },
    dynamics: {
      values: "",
      triggers: "",
      underPressure: "",
      confident: "",
      disengaged: "",
    },
  };
}

export function emptyArbitrePersona(): PersonaData {
  return {
    type: "arbitre",
    identity: { nameTitle: "", quote: "", biography: "" },
    psychology: {
      ocean: { O: 5, C: 5, E: 5, A: 5, N: 5 },
      posture: "ADULTE",
      bias: "",
      blindSpot: "",
    },
    voice: { register: "", syntax: "", tics: "", argumentation: "" },
    moderation: {
      style: "",
      redirection: "",
      whenStagnates: "",
      whenDominates: "",
    },
    dynamics: { underPressure: "", enthusiastic: "" },
  };
}
