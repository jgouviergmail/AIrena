import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Clapperboard, Handshake, Loader2, Plus, RefreshCw, Sparkles, Zap } from "lucide-react";
import { analyseCast, type CastMember, type CompatibilityKind } from "@/lib/casting";
import { extractErrorMessage } from "@/lib/error-utils";
import { getProfileEmoji } from "@/lib/profile-emoji";
import * as api from "@/lib/tauri-api";
import type { CastingSuggestion, PredefinedProfile } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { toast } from "@/stores/useToastStore";
import { SectionLabel } from "./steps/shared";

/** Gladiateurs asked for by default, and the floor of the selector (a debate needs two). */
const DEFAULT_CAST_SIZE = 3;
const MIN_CAST_SIZE = 2;

interface CastingAssistantProps {
  /** Replace the current cast with these profiles (and the moderator when suggested) */
  onReplace: (gladiateurs: PredefinedProfile[], arbitre: PredefinedProfile | null) => void;
  /** Add these profiles to the current cast */
  onAdd: (gladiateurs: PredefinedProfile[]) => void;
}

/**
 * "Casting assisté": one model call proposes a cast suited to the topic, with a
 * reason per pick; the user replaces or completes their selection with it.
 */
export function CastingAssistant({ onReplace, onAdd }: CastingAssistantProps) {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const profiles = useSettingsStore((s) => s.profiles);
  const arbitreProfiles = useSettingsStore((s) => s.arbitreProfiles);
  const engineConstants = useSettingsStore((s) => s.engineConstants);
  const loadEngineConstants = useSettingsStore((s) => s.loadEngineConstants);
  const [count, setCount] = useState(DEFAULT_CAST_SIZE);
  const [loading, setLoading] = useState(false);
  const [suggestion, setSuggestion] = useState<CastingSuggestion | null>(null);

  useEffect(() => { loadEngineConstants(); }, [loadEngineConstants]);
  const maxCount = engineConstants?.castingMaxGladiateurs ?? DEFAULT_CAST_SIZE;
  const hasTopic = topic.trim().length > 0;

  const picks = useMemo(() => {
    if (!suggestion) return [];
    return suggestion.gladiateurs.flatMap((pick) => {
      const profile = profiles.find((p) => p.id === pick.id);
      return profile ? [{ profile, reason: pick.reason }] : [];
    });
  }, [suggestion, profiles]);
  const arbitre = useMemo(
    () => (suggestion?.arbitre ? arbitreProfiles.find((p) => p.id === suggestion.arbitre) ?? null : null),
    [suggestion, arbitreProfiles],
  );

  const suggest = async () => {
    if (!hasTopic || loading) return;
    setLoading(true);
    try {
      setSuggestion(await api.suggestCasting(topic, discussionMode, discussionLanguage, count));
    } catch (e: unknown) {
      toast.error(t("casting.failed"), extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  };

  const profileName = (p: PredefinedProfile) => t(`profiles.${p.id}.name`, { defaultValue: p.name });

  return (
    <div className="space-y-3">
      <SectionLabel icon={Clapperboard}>{t("casting.title")}</SectionLabel>
      <p className="text-xs text-muted-foreground">{t("casting.hint")}</p>
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-2 text-xs text-muted-foreground">
          {t("casting.count")}
          <select
            value={Math.min(count, maxCount)}
            onChange={(e) => setCount(Number(e.target.value))}
            className="rounded-md border border-border bg-background px-2 py-1 text-sm text-foreground"
          >
            {Array.from({ length: Math.max(maxCount - MIN_CAST_SIZE + 1, 1) }, (_, i) => MIN_CAST_SIZE + i).map((n) => (
              <option key={n} value={n}>{n}</option>
            ))}
          </select>
        </label>
        <button
          type="button"
          onClick={suggest}
          disabled={!hasTopic || loading}
          className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground transition-opacity disabled:opacity-50"
        >
          {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Sparkles className="h-4 w-4" />}
          {loading ? t("casting.suggesting") : suggestion ? t("casting.again") : t("casting.suggest")}
        </button>
        {!hasTopic && <span className="text-xs text-amber-600 dark:text-amber-400">{t("casting.needTopic")}</span>}
      </div>

      {suggestion && picks.length === 0 && (
        <p className="text-sm text-muted-foreground">{t("casting.empty")}</p>
      )}
      {picks.length > 0 && (
        <div className="space-y-2 rounded-lg border border-primary/30 bg-primary/5 p-3">
          <ul className="grid gap-2 sm:grid-cols-2">
            {picks.map(({ profile, reason }) => (
              <li key={profile.id} className="rounded-md border border-border bg-card px-3 py-2 text-sm">
                <p className="font-medium text-foreground">
                  <span className="mr-1.5">{getProfileEmoji(profile.name, profile.systemPrompt)}</span>
                  {profileName(profile)}
                </p>
                {reason && <p className="mt-0.5 text-xs text-muted-foreground">{reason}</p>}
              </li>
            ))}
          </ul>
          {arbitre && (
            <p className="text-xs text-muted-foreground">
              <span className="font-medium text-foreground">{t("casting.arbitreSuggested")}:</span> {profileName(arbitre)}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => onReplace(picks.map((p) => p.profile), arbitre)}
              className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
            >
              <RefreshCw className="h-3.5 w-3.5" />
              {t("casting.replace")}
            </button>
            <button
              type="button"
              onClick={() => onAdd(picks.map((p) => p.profile))}
              className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-foreground hover:bg-accent"
            >
              <Plus className="h-3.5 w-3.5" />
              {t("casting.add")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

const KIND_STYLES: Record<CompatibilityKind, { icon: typeof Handshake; className: string }> = {
  allies: { icon: Handshake, className: "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300" },
  friction: { icon: Zap, className: "border-orange-500/40 bg-orange-500/10 text-orange-700 dark:text-orange-300" },
  neutral: { icon: Handshake, className: "border-border bg-card text-muted-foreground" },
};

/** Probable affinities of the selected cast, read locally from the OCEAN scores. */
export function CompatibilityMatrix({ members }: { members: CastMember[] }) {
  const { t } = useTranslation();
  const analysis = useMemo(() => analyseCast(members), [members]);
  if (members.length < MIN_CAST_SIZE) return null;
  const notable = analysis.pairs.filter((p) => p.kind !== "neutral");
  return (
    <div className="space-y-2">
      <SectionLabel icon={Handshake}>{t("casting.compatibilityTitle")}</SectionLabel>
      {analysis.contrastLevel && (
        <p className="text-xs text-muted-foreground">
          <span className="font-medium text-foreground">{t("casting.contrast")}:</span> {t(`casting.contrast_${analysis.contrastLevel}`)}
        </p>
      )}
      {notable.length === 0 && analysis.pairs.length > 0 && (
        <p className="text-xs text-muted-foreground">{t("casting.allNeutral")}</p>
      )}
      {notable.length > 0 && (
        <ul className="flex flex-wrap gap-2">
          {notable.map((p) => {
            const { icon: Icon, className } = KIND_STYLES[p.kind];
            return (
              <li key={`${p.a.id}-${p.b.id}`} className={cn("inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs", className)}>
                <Icon className="h-3 w-3" aria-hidden="true" />
                <span>{p.a.name || "?"} · {p.b.name || "?"}</span>
                <span className="opacity-70">— {t(`casting.${p.kind}`)}</span>
              </li>
            );
          })}
        </ul>
      )}
      {analysis.unscored.length > 0 && (
        <p className="text-[11px] text-muted-foreground">
          {t("casting.noOcean", { names: analysis.unscored.map((m) => m.name || "?").join(", ") })}
        </p>
      )}
    </div>
  );
}
