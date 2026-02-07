import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  ChevronDown,
  ChevronUp,
  Globe,
  Play,
  Plus,
  Trash2,
  UserPlus,
} from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { LlmParamsForm } from "@/components/setup/LlmParamsForm";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useArenaStore } from "@/stores/useArenaStore";
import { cn } from "@/lib/utils";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";
import type { PredefinedProfile } from "@/lib/types";
import * as api from "@/lib/tauri-api";

const TOTAL_STEPS = 4;

export default function SetupPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const step = useSetupStore((s) => s.step);
  const setStep = useSetupStore((s) => s.setStep);
  const topic = useSetupStore((s) => s.topic);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const addGladiateur = useSetupStore((s) => s.addGladiateur);
  const buildConfig = useSetupStore((s) => s.buildConfig);
  const settings = useSettingsStore((s) => s.settings);
  const profiles = useSettingsStore((s) => s.profiles);
  const handleEvent = useArenaStore((s) => s.handleEvent);
  const arenaReset = useArenaStore((s) => s.reset);
  const startingRef = useRef(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canNext = () => {
    switch (step) {
      case 0:
        return topic.trim().length > 0;
      case 1:
        return arbitre.name.trim().length > 0;
      case 2:
        return gladiateurs.length >= 1;
      case 3:
        return true;
      default:
        return false;
    }
  };

  const handleStart = async () => {
    if (startingRef.current) return;
    if (!settings.username.trim()) {
      setError(t("settings.usernameRequired"));
      return;
    }
    startingRef.current = true;
    setStarting(true);
    setError(null);
    arenaReset();
    try {
      const config = buildConfig(settings.username.trim());
      await api.startDiscussion(config, (event) => {
        handleEvent(event);
      });
      navigate("/arena");
    } catch (e: any) {
      setError(e?.message ?? t("errors.generic"));
      setStarting(false);
      startingRef.current = false;
    }
  };

  const addGladiateurFromProfile = (profile: PredefinedProfile) => {
    addGladiateur({
      id: `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name: profile.name,
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: profile.systemPrompt,
      llmParams: { ...DEFAULT_LLM_PARAMS },
    });
  };

  const addEmptyGladiateur = () => {
    addGladiateur({
      id: `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name: "",
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: "",
      llmParams: { ...DEFAULT_LLM_PARAMS },
    });
  };

  return (
    <>
      <TopBar title={t("setup.title")} />
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Stepper indicator */}
        <div className="flex items-center justify-center gap-2 border-b border-border px-4 py-3">
          {Array.from({ length: TOTAL_STEPS }).map((_, i) => (
            <div key={i} className="flex items-center gap-2">
              <div
                className={cn(
                  "flex h-7 w-7 items-center justify-center rounded-full text-xs font-medium transition-colors",
                  i === step
                    ? "bg-primary text-primary-foreground"
                    : i < step
                      ? "bg-primary/20 text-primary"
                      : "bg-muted text-muted-foreground",
                )}
              >
                {i + 1}
              </div>
              {i < TOTAL_STEPS - 1 && (
                <div
                  className={cn(
                    "h-px w-8",
                    i < step ? "bg-primary" : "bg-border",
                  )}
                />
              )}
            </div>
          ))}
          <span className="ml-3 text-xs text-muted-foreground">
            {t("setup.step", { current: step + 1, total: TOTAL_STEPS })}
          </span>
        </div>

        {/* Step content */}
        <div className="flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-2xl">
            {step === 0 && <StepTopic />}
            {step === 1 && <StepArbitre />}
            {step === 2 && (
              <StepGladiateurs
                profiles={profiles}
                onAddFromProfile={addGladiateurFromProfile}
                onAddEmpty={addEmptyGladiateur}
              />
            )}
            {step === 3 && <StepSummary />}
          </div>
        </div>

        {/* Navigation buttons */}
        <div className="flex items-center justify-between border-t border-border px-6 py-3">
          <button
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            className="flex items-center gap-1.5 rounded-md border border-border px-4 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-30"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("setup.previous")}
          </button>

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}

          {step < TOTAL_STEPS - 1 ? (
            <button
              onClick={() => setStep(step + 1)}
              disabled={!canNext()}
              className="flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-30"
            >
              {t("setup.next")}
              <ArrowRight className="h-4 w-4" />
            </button>
          ) : (
            <button
              onClick={handleStart}
              disabled={starting || !canNext()}
              className="flex items-center gap-1.5 rounded-md bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-30"
            >
              <Play className="h-4 w-4" />
              {t("setup.start")}
            </button>
          )}
        </div>
      </div>
    </>
  );
}

function StepTopic() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const setTopic = useSetupStore((s) => s.setTopic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const setDiscussionLanguage = useSetupStore((s) => s.setDiscussionLanguage);

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.topic")}
        </label>
        <textarea
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder={t("setup.topicPlaceholder")}
          rows={4}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.discussionLanguage")}
        </label>
        <div className="flex gap-2">
          {(["fr", "en", "zh"] as const).map((lang) => (
            <button
              key={lang}
              onClick={() => setDiscussionLanguage(lang)}
              className={cn(
                "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
                discussionLanguage === lang
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              <Globe className="h-3.5 w-3.5" />
              {t(`languages.${lang}`)}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function StepArbitre() {
  const { t } = useTranslation();
  const arbitre = useSetupStore((s) => s.arbitre);
  const updateArbitre = useSetupStore((s) => s.updateArbitre);
  const updateArbitreLlm = useSetupStore((s) => s.updateArbitreLlm);
  const [showLlm, setShowLlm] = useState(false);

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.arbitreName")}
        </label>
        <input
          type="text"
          value={arbitre.name}
          onChange={(e) => updateArbitre({ name: e.target.value })}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.arbitrePrompt")}
        </label>
        <textarea
          value={arbitre.systemPrompt}
          onChange={(e) => updateArbitre({ systemPrompt: e.target.value })}
          rows={4}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.turnDistribution")}
        </label>
        <div className="flex gap-2">
          {(["sequential", "random"] as const).map((dist) => (
            <button
              key={dist}
              onClick={() => updateArbitre({ turnDistribution: dist })}
              className={cn(
                "rounded-md border px-3 py-1.5 text-sm transition-colors",
                arbitre.turnDistribution === dist
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              {t(`setup.${dist}`)}
            </button>
          ))}
        </div>
      </div>

      <button
        onClick={() => setShowLlm(!showLlm)}
        className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
      >
        {showLlm ? (
          <ChevronUp className="h-4 w-4" />
        ) : (
          <ChevronDown className="h-4 w-4" />
        )}
        {t("setup.llmParams")}
      </button>
      {showLlm && (
        <LlmParamsForm
          params={arbitre.llmParams}
          onChange={updateArbitreLlm}
        />
      )}
    </div>
  );
}

function StepGladiateurs({
  profiles,
  onAddFromProfile,
  onAddEmpty,
}: {
  profiles: PredefinedProfile[];
  onAddFromProfile: (p: PredefinedProfile) => void;
  onAddEmpty: () => void;
}) {
  const { t } = useTranslation();
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const removeGladiateur = useSetupStore((s) => s.removeGladiateur);
  const updateGladiateur = useSetupStore((s) => s.updateGladiateur);
  const updateGladiateurLlm = useSetupStore((s) => s.updateGladiateurLlm);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  return (
    <div className="space-y-4">
      {/* Profile picker */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.selectProfile")}
        </label>
        <div className="flex flex-wrap gap-2">
          {profiles.map((p) => (
            <button
              key={p.id}
              onClick={() => onAddFromProfile(p)}
              className="rounded-md border border-border px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent"
            >
              <UserPlus className="mr-1.5 inline h-3.5 w-3.5" />
              {p.name}
            </button>
          ))}
          <button
            onClick={onAddEmpty}
            className="rounded-md border border-dashed border-border px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <Plus className="mr-1.5 inline h-3.5 w-3.5" />
            {t("setup.addGladiateur")}
          </button>
        </div>
      </div>

      {gladiateurs.length === 0 && (
        <p className="py-8 text-center text-sm text-muted-foreground">
          {t("setup.minGladiateurs")}
        </p>
      )}

      {/* Gladiateur cards */}
      <div className="space-y-3">
        {gladiateurs.map((g, idx) => (
          <div
            key={g.id}
            className="rounded-lg border border-border bg-card p-4"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
                  {idx + 1}
                </span>
                <input
                  type="text"
                  value={g.name}
                  onChange={(e) =>
                    updateGladiateur(g.id, { name: e.target.value })
                  }
                  placeholder={t("setup.gladiateurName")}
                  className="border-none bg-transparent text-sm font-medium text-foreground placeholder:text-muted-foreground focus:outline-none"
                />
              </div>
              <div className="flex items-center gap-1">
                <button
                  onClick={() =>
                    setExpandedId(expandedId === g.id ? null : g.id)
                  }
                  className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  {expandedId === g.id ? (
                    <ChevronUp className="h-4 w-4" />
                  ) : (
                    <ChevronDown className="h-4 w-4" />
                  )}
                </button>
                <button
                  onClick={() => removeGladiateur(g.id)}
                  className="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>

            {expandedId === g.id && (
              <div className="mt-3 space-y-3 border-t border-border pt-3">
                <div className="space-y-1.5">
                  <label className="text-xs text-muted-foreground">
                    {t("setup.gladiateurPrompt")}
                  </label>
                  <textarea
                    value={g.systemPrompt}
                    onChange={(e) =>
                      updateGladiateur(g.id, {
                        systemPrompt: e.target.value,
                      })
                    }
                    rows={3}
                    className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                </div>
                <div>
                  <label className="mb-2 block text-xs text-muted-foreground">
                    {t("setup.llmParams")}
                  </label>
                  <LlmParamsForm
                    params={g.llmParams}
                    onChange={(patch) => updateGladiateurLlm(g.id, patch)}
                  />
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function StepSummary() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const setMaxTurns = useSetupStore((s) => s.setMaxTurns);
  const userInterventionTimeoutSecs = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const setUserTimeout = useSetupStore((s) => s.setUserTimeout);

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-border bg-card p-4 space-y-3">
        <SummaryRow label={t("setup.topic")} value={topic} />
        <SummaryRow
          label={t("setup.discussionLanguage")}
          value={t(`languages.${discussionLanguage}`)}
        />
        <SummaryRow label={t("setup.arbitreName")} value={arbitre.name} />
        <SummaryRow
          label={t("setup.turnDistribution")}
          value={t(`setup.${arbitre.turnDistribution}`)}
        />
        <SummaryRow
          label={t("setup.gladiateurs")}
          value={gladiateurs.map((g) => g.name).join(", ") || "-"}
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">
            {t("setup.maxTurns")}
          </label>
          <input
            type="number"
            min={1}
            max={100}
            value={maxTurns ?? ""}
            onChange={(e) =>
              setMaxTurns(e.target.value ? parseInt(e.target.value) : null)
            }
            placeholder={t("setup.maxTurnsPlaceholder")}
            className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">
            {t("setup.userTimeout")}
          </label>
          <input
            type="number"
            min={30}
            max={600}
            value={userInterventionTimeoutSecs}
            onChange={(e) => setUserTimeout(parseInt(e.target.value) || 120)}
            className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-sm text-muted-foreground">{label}</span>
      <span className="text-sm font-medium text-foreground">{value}</span>
    </div>
  );
}
