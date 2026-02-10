import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Code, FormInput } from "lucide-react";
import { parsePersona, emptyGladiateurPersona, emptyArbitrePersona } from "@/lib/persona-parser";
import { serializePersona } from "@/lib/persona-serializer";
import type { Lang } from "@/lib/persona-labels";
import type { PersonaData, PostureValue, OceanScores } from "@/lib/persona-types";
import { OceanSliders } from "./OceanSliders";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface PersonaEditorProps {
  systemPrompt: string;
  profileType: "gladiateur" | "arbitre";
  discussionLanguage: string;
  onChange: (newPrompt: string) => void;
}

// ---------------------------------------------------------------------------
// Posture options (canonical FR values used internally)
// ---------------------------------------------------------------------------

const POSTURE_OPTIONS: PostureValue[] = [
  "ADULTE",
  "ENFANT_LIBRE",
  "ENFANT_ADAPTÉ",
  "PARENT_CRITIQUE",
  "PARENT_NOURRICIER",
];

// ---------------------------------------------------------------------------
// Accordion section helper
// ---------------------------------------------------------------------------

function Section({
  id,
  icon,
  title,
  open,
  onToggle,
  children,
}: {
  id: string;
  icon: string;
  title: string;
  open: boolean;
  onToggle: (id: string) => void;
  children: React.ReactNode;
}) {
  return (
    <div className="border-b border-border last:border-b-0">
      <button
        type="button"
        onClick={() => onToggle(id)}
        className="flex w-full items-center gap-2 py-2.5 text-sm font-medium text-foreground hover:text-primary transition-colors"
      >
        {open ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <span>{icon}</span>
        <span>{title}</span>
      </button>
      {open && <div className="pb-3 pl-6 pr-1 space-y-3">{children}</div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Field helpers
// ---------------------------------------------------------------------------

function FieldInput({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-muted-foreground">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
      />
    </div>
  );
}

function FieldTextarea({
  label,
  value,
  onChange,
  rows = 2,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  rows?: number;
  placeholder?: string;
}) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-muted-foreground">{label}</label>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        rows={rows}
        placeholder={placeholder}
        className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PersonaEditor({
  systemPrompt,
  profileType,
  discussionLanguage,
  onChange,
}: PersonaEditorProps) {
  const { t } = useTranslation();
  const lang: Lang = discussionLanguage === "en" || discussionLanguage === "zh" ? discussionLanguage : "fr";

  // Parsed persona data
  const [persona, setPersona] = useState<PersonaData | null>(null);
  const [rawMode, setRawMode] = useState(false);
  const [rawText, setRawText] = useState(systemPrompt);

  // Track last serialized XML to avoid re-parse loops
  // Initialized to null so the first useEffect always runs (even when systemPrompt is "")
  const lastSerializedRef = useRef<string | null>(null);

  // Accordion state — identity open by default
  const [openSections, setOpenSections] = useState<Set<string>>(
    new Set(["identity"]),
  );

  const toggleSection = (id: string) => {
    setOpenSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // Parse systemPrompt on mount or when it changes externally
  useEffect(() => {
    // Skip re-parse if this change came from our own serialization
    if (systemPrompt === lastSerializedRef.current) return;

    const result = parsePersona(systemPrompt, profileType);
    if (result.success) {
      setPersona(result.data);
      setRawMode(false);
    } else if (systemPrompt.trim() === "") {
      // Empty prompt — start with default empty persona
      const empty = profileType === "arbitre" ? emptyArbitrePersona() : emptyGladiateurPersona();
      setPersona(empty);
      setRawMode(false);
    } else {
      // Non-XML prompt — fallback to raw mode
      setPersona(null);
      setRawMode(true);
    }
    setRawText(systemPrompt);
  }, [systemPrompt, profileType]);

  // Re-serialize when discussion language changes
  useEffect(() => {
    if (persona) {
      const xml = serializePersona(persona, lang);
      lastSerializedRef.current = xml;
      onChange(xml);
    }
  // Only re-serialize when lang changes, not on every persona change
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lang]);

  // Update a field in the persona and serialize
  const updatePersona = (updater: (prev: PersonaData) => PersonaData) => {
    if (!persona) return;
    const updated = updater(persona);
    setPersona(updated);
    const xml = serializePersona(updated, lang);
    lastSerializedRef.current = xml;
    onChange(xml);
  };

  // Raw mode handlers
  const handleRawChange = (text: string) => {
    setRawText(text);
    onChange(text);
  };

  const switchToStructured = () => {
    const result = parsePersona(rawText, profileType);
    if (result.success) {
      setPersona(result.data);
      setRawMode(false);
    }
  };

  const switchToRaw = () => {
    if (persona) {
      const xml = serializePersona(persona, lang);
      setRawText(xml);
    }
    setRawMode(true);
  };

  // ---- Raw mode UI ----
  if (rawMode) {
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">
            {t("personaEditor.rawMode")}
          </span>
          <button
            type="button"
            onClick={switchToStructured}
            className="flex items-center gap-1 text-xs text-primary hover:text-primary/80"
          >
            <FormInput className="h-3 w-3" />
            {t("personaEditor.structuredMode")}
          </button>
        </div>
        <textarea
          value={rawText}
          onChange={(e) => handleRawChange(e.target.value)}
          rows={8}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>
    );
  }

  if (!persona) return null;

  const { identity, psychology, voice } = persona;

  // ---- Structured mode UI ----
  return (
    <div className="space-y-1">
      {/* Toggle to raw */}
      <div className="flex justify-end">
        <button
          type="button"
          onClick={switchToRaw}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary"
        >
          <Code className="h-3 w-3" />
          {t("personaEditor.rawMode")}
        </button>
      </div>

      <div className="rounded-md border border-border">
        {/* Identity */}
        <Section
          id="identity"
          icon="🎭"
          title={t("personaEditor.identity")}
          open={openSections.has("identity")}
          onToggle={toggleSection}
        >
          <FieldInput
            label={t("personaEditor.nameTitle")}
            value={identity.nameTitle}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                identity: { ...p.identity, nameTitle: v },
              }))
            }
            placeholder={t("personaEditor.nameTitlePlaceholder")}
          />
          <FieldInput
            label={t("personaEditor.quote")}
            value={identity.quote}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                identity: { ...p.identity, quote: v },
              }))
            }
            placeholder={t("personaEditor.quotePlaceholder")}
          />
          <FieldTextarea
            label={t("personaEditor.biography")}
            value={identity.biography}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                identity: { ...p.identity, biography: v },
              }))
            }
            rows={3}
            placeholder={t("personaEditor.biographyPlaceholder")}
          />
        </Section>

        {/* Psychology */}
        <Section
          id="psychology"
          icon="🧠"
          title={t("personaEditor.psychology")}
          open={openSections.has("psychology")}
          onToggle={toggleSection}
        >
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              {t("personaEditor.oceanTitle")}
            </label>
            <OceanSliders
              ocean={psychology.ocean}
              onChange={(ocean: OceanScores) =>
                updatePersona((p) => ({
                  ...p,
                  psychology: { ...p.psychology, ocean },
                }))
              }
            />
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">
              {t("personaEditor.posture")}
            </label>
            <select
              value={psychology.posture}
              onChange={(e) =>
                updatePersona((p) => ({
                  ...p,
                  psychology: {
                    ...p.psychology,
                    posture: e.target.value as PostureValue,
                  },
                }))
              }
              className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            >
              {POSTURE_OPTIONS.map((val) => (
                <option key={val} value={val}>
                  {t(`personaEditor.posture_${val}`)}
                </option>
              ))}
            </select>
          </div>
          <FieldTextarea
            label={t("personaEditor.bias")}
            value={psychology.bias}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                psychology: { ...p.psychology, bias: v },
              }))
            }
            placeholder={t("personaEditor.biasPlaceholder")}
          />
          <FieldTextarea
            label={t("personaEditor.blindSpot")}
            value={psychology.blindSpot}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                psychology: { ...p.psychology, blindSpot: v },
              }))
            }
            placeholder={t("personaEditor.blindSpotPlaceholder")}
          />
        </Section>

        {/* Voice */}
        <Section
          id="voice"
          icon="🗣️"
          title={t("personaEditor.voice")}
          open={openSections.has("voice")}
          onToggle={toggleSection}
        >
          <FieldInput
            label={t("personaEditor.register")}
            value={voice.register}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                voice: { ...p.voice, register: v },
              }))
            }
            placeholder={t("personaEditor.registerPlaceholder")}
          />
          <FieldTextarea
            label={t("personaEditor.syntax")}
            value={voice.syntax}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                voice: { ...p.voice, syntax: v },
              }))
            }
            placeholder={t("personaEditor.syntaxPlaceholder")}
          />
          <FieldTextarea
            label={t("personaEditor.tics")}
            value={voice.tics}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                voice: { ...p.voice, tics: v },
              }))
            }
            placeholder={t("personaEditor.ticsPlaceholder")}
          />
          <FieldTextarea
            label={t("personaEditor.argumentation")}
            value={voice.argumentation}
            onChange={(v) =>
              updatePersona((p) => ({
                ...p,
                voice: { ...p.voice, argumentation: v },
              }))
            }
            placeholder={t("personaEditor.argumentationPlaceholder")}
          />
        </Section>

        {/* Moderation (arbitre only) */}
        {persona.type === "arbitre" && (
          <Section
            id="moderation"
            icon="🛡️"
            title={t("personaEditor.moderation")}
            open={openSections.has("moderation")}
            onToggle={toggleSection}
          >
            <FieldTextarea
              label={t("personaEditor.moderationStyle")}
              value={persona.moderation.style}
              onChange={(v) =>
                updatePersona((p) => {
                  if (p.type !== "arbitre") return p;
                  return { ...p, moderation: { ...p.moderation, style: v } };
                })
              }
              placeholder={t("personaEditor.moderationStylePlaceholder")}
            />
            <FieldTextarea
              label={t("personaEditor.redirection")}
              value={persona.moderation.redirection}
              onChange={(v) =>
                updatePersona((p) => {
                  if (p.type !== "arbitre") return p;
                  return { ...p, moderation: { ...p.moderation, redirection: v } };
                })
              }
              placeholder={t("personaEditor.redirectionPlaceholder")}
            />
            <FieldTextarea
              label={t("personaEditor.whenStagnates")}
              value={persona.moderation.whenStagnates}
              onChange={(v) =>
                updatePersona((p) => {
                  if (p.type !== "arbitre") return p;
                  return { ...p, moderation: { ...p.moderation, whenStagnates: v } };
                })
              }
              placeholder={t("personaEditor.whenStagnatesPlaceholder")}
            />
            <FieldTextarea
              label={t("personaEditor.whenDominates")}
              value={persona.moderation.whenDominates}
              onChange={(v) =>
                updatePersona((p) => {
                  if (p.type !== "arbitre") return p;
                  return { ...p, moderation: { ...p.moderation, whenDominates: v } };
                })
              }
              placeholder={t("personaEditor.whenDominatesPlaceholder")}
            />
          </Section>
        )}

        {/* Dynamics */}
        <Section
          id="dynamics"
          icon="⚡"
          title={t("personaEditor.dynamics")}
          open={openSections.has("dynamics")}
          onToggle={toggleSection}
        >
          {persona.type === "gladiateur" ? (
            <>
              <FieldInput
                label={t("personaEditor.values")}
                value={persona.dynamics.values}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "gladiateur") return p;
                    return { ...p, dynamics: { ...p.dynamics, values: v } };
                  })
                }
                placeholder={t("personaEditor.valuesPlaceholder")}
              />
              <FieldInput
                label={t("personaEditor.triggers")}
                value={persona.dynamics.triggers}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "gladiateur") return p;
                    return { ...p, dynamics: { ...p.dynamics, triggers: v } };
                  })
                }
                placeholder={t("personaEditor.triggersPlaceholder")}
              />
              <FieldTextarea
                label={t("personaEditor.underPressure")}
                value={persona.dynamics.underPressure}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "gladiateur") return p;
                    return { ...p, dynamics: { ...p.dynamics, underPressure: v } };
                  })
                }
                placeholder={t("personaEditor.underPressurePlaceholder")}
              />
              <FieldTextarea
                label={t("personaEditor.confident")}
                value={persona.dynamics.confident}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "gladiateur") return p;
                    return { ...p, dynamics: { ...p.dynamics, confident: v } };
                  })
                }
                placeholder={t("personaEditor.confidentPlaceholder")}
              />
              <FieldTextarea
                label={t("personaEditor.disengaged")}
                value={persona.dynamics.disengaged}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "gladiateur") return p;
                    return { ...p, dynamics: { ...p.dynamics, disengaged: v } };
                  })
                }
                placeholder={t("personaEditor.disengagedPlaceholder")}
              />
            </>
          ) : (
            <>
              <FieldTextarea
                label={t("personaEditor.underPressure")}
                value={persona.dynamics.underPressure}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "arbitre") return p;
                    return { ...p, dynamics: { ...p.dynamics, underPressure: v } };
                  })
                }
                placeholder={t("personaEditor.underPressurePlaceholder")}
              />
              <FieldTextarea
                label={t("personaEditor.enthusiastic")}
                value={persona.dynamics.enthusiastic}
                onChange={(v) =>
                  updatePersona((p) => {
                    if (p.type !== "arbitre") return p;
                    return { ...p, dynamics: { ...p.dynamics, enthusiastic: v } };
                  })
                }
                placeholder={t("personaEditor.enthusiasticPlaceholder")}
              />
            </>
          )}
        </Section>
      </div>
    </div>
  );
}
