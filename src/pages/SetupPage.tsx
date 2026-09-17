import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowLeft, ArrowRight, KeyRound, Play } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { StepTopic } from "@/components/setup/steps/StepTopic";
import { StepArbitre } from "@/components/setup/steps/StepArbitre";
import { StepGladiateurs } from "@/components/setup/steps/StepGladiateurs";
import { StepKnowledge } from "@/components/setup/steps/StepKnowledge";
import { StepSummary } from "@/components/setup/steps/StepSummary";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useArenaStore } from "@/stores/useArenaStore";
import { cn } from "@/lib/utils";
import { applyGlobalContext } from "@/lib/discussion-config";
import type { LicenseStatus } from "@/lib/types";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";

const TOTAL_STEPS = 5;

export default function SetupPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const step = useSetupStore((s) => s.step);
  const setStep = useSetupStore((s) => s.setStep);
  const topic = useSetupStore((s) => s.topic);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const buildConfig = useSetupStore((s) => s.buildConfig);
  const tokenBudgetPreview = useSetupStore((s) => s.tokenBudgetPreview);
  const settings = useSettingsStore((s) => s.settings);
  const handleEvent = useArenaStore((s) => s.handleEvent);
  const arenaReset = useArenaStore((s) => s.reset);
  const startingRef = useRef(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus | null>(null);
  const [budgetExhausted, setBudgetExhausted] = useState(false);
  const licenseValid = licenseStatus?.valid === true;

  useEffect(() => {
    api.checkLicenseStatus().then(setLicenseStatus).catch(() => {});
  }, []);

  // Cloud spend cap: mirror the backend pre-flight so the reason is visible before clicking
  useEffect(() => {
    if (settings.llmProvider !== "deepseek" || !settings.deepseekApiKey.trim()) {
      setBudgetExhausted(false);
      return;
    }
    let cancelled = false;
    api.getLlmUsagePeriod()
      .then((p) => { if (!cancelled) setBudgetExhausted(p.budgetUsd > 0 && p.usage.costUsd >= p.budgetUsd); })
      .catch(() => { if (!cancelled) setBudgetExhausted(false); });
    return () => { cancelled = true; };
  }, [settings.llmProvider, settings.deepseekApiKey, settings.deepseekMonthlyBudgetUsd]);

  // Provider readiness: a cloud provider needs its key (and headroom), a local one needs a model,
  // an OpenAI-compatible server needs its URL and a model
  const providerBlocker: string | null =
    settings.llmProvider === "deepseek"
      ? !settings.deepseekApiKey.trim()
        ? t("setup.deepseekKeyRequired")
        : budgetExhausted
          ? t("setup.budgetExhausted")
          : null
      : settings.llmProvider === "openaiCompat"
        ? settings.openaiCompatBaseUrl.trim() && settings.openaiCompatModel.trim() ? null : t("setup.openaiCompatRequired")
        : settings.ollamaModel ? null : t("setup.ollamaModelRequired");

  const canNext = () => {
    switch (step) {
      case 0:
        return topic.trim().length > 0;
      case 1:
        return arbitre.name.trim().length > 0;
      case 2:
        return gladiateurs.length >= 1;
      case 3:
        return true; // Knowledge step — always passable
      case 4:
        return tokenBudgetPreview?.qualityLevel !== "insufficient";
      default:
        return false;
    }
  };

  const handleStart = async () => {
    if (startingRef.current) return;
    if (!licenseValid) {
      setError(t("setup.licenseRequired"));
      return;
    }
    if (!settings.username.trim()) {
      setError(t("settings.usernameRequired"));
      return;
    }
    if (providerBlocker) {
      setError(providerBlocker);
      return;
    }
    const preview = useSetupStore.getState().tokenBudgetPreview;
    if (preview?.qualityLevel === "insufficient") {
      setError(t("setup.budgetQualityInsufficient"));
      return;
    }
    startingRef.current = true;
    setStarting(true);
    setError(null);
    arenaReset();
    try {
      const config = applyGlobalContext(buildConfig(settings.username.trim()), settings.numCtx);
      await api.startDiscussion(config, handleEvent);
      navigate("/arena");
    } catch (e: unknown) {
      const msg = extractErrorMessage(e);
      setError(msg || t("errors.generic"));
      toast.error(t("setup.startError"), msg);
      setStarting(false);
      startingRef.current = false;
    }
  };

  return (
    <>
      <TopBar title={t("setup.title")} />
      {!licenseValid && licenseStatus !== null && (
        <Banner>{t("setup.licenseRequired")}</Banner>
      )}
      {providerBlocker && <Banner>{providerBlocker}</Banner>}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Stepper indicator */}
        <div className="flex flex-wrap items-center justify-center gap-2 border-b border-border px-4 py-3">
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
                <div className={cn("h-px w-4 sm:w-8", i < step ? "bg-primary" : "bg-border")} />
              )}
            </div>
          ))}
          <span className="ml-3 text-sm font-semibold text-foreground">
            {t("setup.step", { current: step + 1, total: TOTAL_STEPS })}
            {" — "}
            {t(`setup.stepTitle_${step}`)}
          </span>
        </div>

        {/* Step content */}
        <div className="flex-1 overflow-y-auto p-4 sm:p-6">
          <div className="mx-auto max-w-2xl">
            {step === 0 && <StepTopic />}
            {step === 1 && <StepArbitre />}
            {step === 2 && <StepGladiateurs />}
            {step === 3 && <StepKnowledge />}
            {step === 4 && <StepSummary />}
          </div>
        </div>

        {/* Navigation buttons */}
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border px-4 py-3 sm:px-6">
          <button
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            className="flex items-center gap-1.5 rounded-md border border-border px-4 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-30"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("setup.previous")}
          </button>

          {error && <p className="text-sm text-destructive">{error}</p>}

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
              disabled={starting || !canNext() || !licenseValid || !!providerBlocker}
              title={providerBlocker ?? undefined}
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

function Banner({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2 border-b border-destructive/20 bg-destructive/5 px-4 py-2 text-sm text-destructive">
      <KeyRound className="h-4 w-4 shrink-0" />
      {children}
    </div>
  );
}
