// ---- Posture (Transactional Analysis) ----
export type PostureValue =
  | "ADULTE"
  | "ENFANT_LIBRE"
  | "ENFANT_ADAPTÉ"
  | "PARENT_CRITIQUE"
  | "PARENT_NOURRICIER";

// ---- OCEAN Big Five ----
export interface OceanScores {
  O: number; // Openness 1-10
  C: number; // Conscientiousness 1-10
  E: number; // Extraversion 1-10
  A: number; // Agreeableness 1-10
  N: number; // Neuroticism 1-10
}

// ---- Identity section ----
export interface IdentityData {
  nameTitle: string; // "Le Scientifique — Chercheur pluridisciplinaire"
  quote: string; // Without surrounding quotes
  biography: string; // Free text, 2-3 sentences
}

// ---- Psychology section ----
export interface PsychologyData {
  ocean: OceanScores;
  posture: PostureValue;
  bias: string; // Full line: "Appel à l'autorité — description..."
  blindSpot: string; // Full line: "Biais de complexité — description..."
}

// ---- Voice section ----
export interface VoiceData {
  register: string; // Full text: "SOUTENU, TECHNIQUE"
  syntax: string;
  tics: string; // Full text with "quoted phrases"
  argumentation: string;
}

// ---- Gladiateur dynamics ----
export interface DynamicsData {
  values: string;
  triggers: string;
  underPressure: string;
  confident: string;
  disengaged: string;
}

// ---- Arbitre moderation ----
export interface ModerationData {
  style: string;
  redirection: string;
  whenStagnates: string;
  whenDominates: string;
}

// ---- Arbitre dynamics (abridged) ----
export interface ArbitreDynamicsData {
  underPressure: string;
  enthusiastic: string;
}

// ---- Persona variants ----
export interface GladiateurPersona {
  type: "gladiateur";
  identity: IdentityData;
  psychology: PsychologyData;
  voice: VoiceData;
  dynamics: DynamicsData;
}

export interface ArbitrePersona {
  type: "arbitre";
  identity: IdentityData;
  psychology: PsychologyData;
  voice: VoiceData;
  moderation: ModerationData;
  dynamics: ArbitreDynamicsData;
}

export type PersonaData = GladiateurPersona | ArbitrePersona;

export type ParseResult =
  | { success: true; data: PersonaData }
  | { success: false; raw: string };
