import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cloud, Coins, Eye, EyeOff, KeyRound, Layers, Loader2, RefreshCw, RotateCcw, Wallet } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/lib/error-utils";
import { formatUsd } from "@/lib/cost-estimate";
import { toast } from "@/stores/useToastStore";
import type { DeepSeekBalance, LlmUsagePeriod } from "@/lib/types";
import * as api from "@/lib/tauri-api";
import { buttonClass, Explainer, Field, inputClass, Section, secretInputClass, StatusPill } from "./SettingsPrimitives";

/**
 * DeepSeek account, model, context budget and monthly spend.
 * The API key is NOT auto-saved while typing: it is validated against the
 * account first, then stored — a mistyped key never silently replaces a good one.
 */
export function DeepSeekSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const llmConstants = useSettingsStore((s) => s.llmConstants);
  const loadLlmConstants = useSettingsStore((s) => s.loadLlmConstants);

  const [keyDraft, setKeyDraft] = useState(settings.deepseekApiKey);
  const [showKey, setShowKey] = useState(false);
  const [validating, setValidating] = useState(false);
  const [balance, setBalance] = useState<DeepSeekBalance | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [modelsFromApi, setModelsFromApi] = useState<boolean | null>(null);
  const [loadingModels, setLoadingModels] = useState(false);
  const [period, setPeriod] = useState<LlmUsagePeriod | null>(null);
  const [resetting, setResetting] = useState(false);

  useEffect(() => {
    loadLlmConstants();
  }, [loadLlmConstants]);

  // Keep the draft in sync when settings hydrate after mount
  useEffect(() => {
    setKeyDraft(settings.deepseekApiKey);
  }, [settings.deepseekApiKey]);

  const refreshPeriod = useCallback(async () => {
    try {
      setPeriod(await api.getLlmUsagePeriod());
    } catch (e: unknown) {
      toast.error(t("settings.deepseekPeriodError"), extractErrorMessage(e));
    }
  }, [t]);

  useEffect(() => {
    refreshPeriod();
  }, [refreshPeriod]);

  const refreshModels = useCallback(async (key?: string) => {
    if (!(key ?? settings.deepseekApiKey).trim()) return;
    setLoadingModels(true);
    try {
      const res = await api.listDeepSeekModels(key);
      setModels(res.models);
      setModelsFromApi(res.fromApi);
      if (!settings.deepseekModel && res.models.length > 0) {
        const preferred = llmConstants?.deepseekDefaultModel;
        updateSettings({ deepseekModel: preferred && res.models.includes(preferred) ? preferred : res.models[0] });
      }
    } catch (e: unknown) {
      toast.error(t("settings.deepseekModelsError"), extractErrorMessage(e));
    } finally {
      setLoadingModels(false);
    }
  }, [settings.deepseekApiKey, settings.deepseekModel, llmConstants, updateSettings, t]);

  // Saved key → list models once
  useEffect(() => {
    if (settings.deepseekApiKey.trim() && models.length === 0) {
      refreshModels();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.deepseekApiKey]);

  const keyDirty = keyDraft.trim() !== settings.deepseekApiKey.trim();

  const handleValidateAndSave = async () => {
    const key = keyDraft.trim();
    if (!key) return;
    setValidating(true);
    try {
      const res = await api.validateDeepSeekKey(key);
      setBalance(res);
      updateSettings({ deepseekApiKey: key });
      toast.success(t("settings.deepseekKeyValid"));
      await refreshModels(key);
    } catch (e: unknown) {
      setBalance(null);
      toast.error(t("settings.deepseekKeyInvalid"), extractErrorMessage(e));
    } finally {
      setValidating(false);
    }
  };

  const handleForgetKey = () => {
    setKeyDraft("");
    setBalance(null);
    setModels([]);
    setModelsFromApi(null);
    updateSettings({ deepseekApiKey: "" });
  };

  const handleResetPeriod = async () => {
    setResetting(true);
    try {
      setPeriod(await api.resetLlmUsagePeriod());
      toast.success(t("settings.deepseekPeriodResetDone"));
    } catch (e: unknown) {
      toast.error(t("settings.deepseekPeriodError"), extractErrorMessage(e));
    } finally {
      setResetting(false);
    }
  };

  // Context budget: committed on blur/Enter (clamped to backend bounds)
  const minCtx = llmConstants?.deepseekMinContextBudget ?? 4096;
  const maxCtx = llmConstants?.deepseekMaxContextBudget ?? 262144;
  const [ctxInput, setCtxInput] = useState(String(settings.numCtx));
  useEffect(() => setCtxInput(String(settings.numCtx)), [settings.numCtx]);
  const commitCtx = () => {
    const parsed = parseInt(ctxInput) || (llmConstants?.deepseekDefaultContextBudget ?? minCtx);
    const clamped = Math.max(minCtx, Math.min(maxCtx, parsed));
    setCtxInput(String(clamped));
    updateSettings({ numCtx: clamped });
  };

  const hasKey = settings.deepseekApiKey.trim().length > 0;
  const spent = period?.usage.costUsd ?? 0;
  // Live value: the gauge follows the cap being typed, not the one persisted at mount
  const budget = settings.deepseekMonthlyBudgetUsd;
  const ratio = budget > 0 ? Math.min(1, spent / budget) : 0;
  const warnRatio = llmConstants?.budgetWarnRatio ?? 0.8;

  return (
    <Section title={t("settings.deepseek")} icon={Cloud}>
      {/* API key */}
      <Field label={<><KeyRound className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.deepseekApiKey")}</>}>
        <Explainer>{t("settings.deepseekApiKeyDesc")}</Explainer>
        <div className="flex flex-wrap gap-2">
          <div className="relative min-w-48 flex-1">
            <input
              type={showKey ? "text" : "password"}
              value={keyDraft}
              onChange={(e) => setKeyDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleValidateAndSave()}
              placeholder="sk-..."
              autoComplete="off"
              className={secretInputClass}
            />
            <button
              type="button"
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
          <button onClick={handleValidateAndSave} disabled={validating || !keyDraft.trim()} className={buttonClass}>
            {validating ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <KeyRound className="h-3.5 w-3.5" />}
            {keyDirty ? t("settings.deepseekValidateSave") : t("settings.deepseekValidate")}
          </button>
          {hasKey && (
            <button onClick={handleForgetKey} className={cn(buttonClass, "text-destructive hover:bg-destructive/10")}>
              {t("settings.deepseekForgetKey")}
            </button>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {!hasKey ? (
            <StatusPill tone="muted">{t("settings.deepseekNoKey")}</StatusPill>
          ) : keyDirty ? (
            <StatusPill tone="warn">{t("settings.deepseekKeyUnsaved")}</StatusPill>
          ) : (
            <StatusPill tone="ok">{t("settings.deepseekKeySaved")}</StatusPill>
          )}
          {balance && (
            <span className="text-xs text-muted-foreground">
              {balance.isAvailable
                ? t("settings.deepseekBalance", {
                    amount: balance.balances.map((b) => `${b.totalBalance} ${b.currency}`).join(", ") || "—",
                  })
                : t("settings.deepseekBalanceUnavailable")}
            </span>
          )}
        </div>
        <p className="text-xs text-muted-foreground">
          <a href="https://platform.deepseek.com" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
            {t("settings.deepseekLink")}
          </a>
        </p>
      </Field>

      {hasKey && (
        <>
          {/* Model */}
          <Field label={t("settings.deepseekModel")}>
            <Explainer>{t("settings.deepseekModelDesc")}</Explainer>
            <div className="flex flex-wrap items-center gap-2">
              <select
                value={settings.deepseekModel}
                onChange={(e) => updateSettings({ deepseekModel: e.target.value })}
                disabled={loadingModels}
                className={`${inputClass} min-w-48 flex-1`}
              >
                {!models.includes(settings.deepseekModel) && settings.deepseekModel && (
                  <option value={settings.deepseekModel}>{settings.deepseekModel}</option>
                )}
                {models.map((m) => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
              <button onClick={() => refreshModels()} disabled={loadingModels} className={buttonClass} title={t("settings.deepseekRefreshModels")}>
                {loadingModels ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
              </button>
            </div>
            {modelsFromApi === false && (
              <p className="text-xs text-amber-500">{t("settings.deepseekModelsFallback")}</p>
            )}
          </Field>

          {/* Context budget */}
          <Field label={<><Layers className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.deepseekContextBudget")}</>}>
            <Explainer>{t("settings.deepseekContextBudgetDesc", { min: minCtx.toLocaleString(), max: maxCtx.toLocaleString() })}</Explainer>
            <div className="flex flex-wrap items-center gap-3">
              <input
                type="number"
                min={minCtx}
                max={maxCtx}
                value={ctxInput}
                onChange={(e) => setCtxInput(e.target.value)}
                onBlur={commitCtx}
                onKeyDown={(e) => e.key === "Enter" && commitCtx()}
                className={`${inputClass} w-32 font-mono`}
              />
              <span className="text-sm text-muted-foreground">tokens</span>
              {llmConstants && (
                <button
                  onClick={() => updateSettings({ numCtx: llmConstants.deepseekDefaultContextBudget })}
                  className="rounded-md border border-primary/30 bg-primary/5 px-2 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  {t("settings.deepseekContextDefault", { value: llmConstants.deepseekDefaultContextBudget.toLocaleString() })}
                </button>
              )}
            </div>
          </Field>

          {/* Monthly budget + period */}
          <Field label={<><Wallet className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.deepseekMonthlyBudget")}</>}>
            <Explainer>{t("settings.deepseekMonthlyBudgetDesc", { warn: Math.round(warnRatio * 100) })}</Explainer>
            <div className="flex flex-wrap items-center gap-3">
              <input
                type="number"
                min={0}
                step={1}
                value={settings.deepseekMonthlyBudgetUsd}
                onChange={(e) => updateSettings({ deepseekMonthlyBudgetUsd: Math.max(0, parseFloat(e.target.value) || 0) })}
                className={`${inputClass} w-32 font-mono`}
              />
              <span className="text-sm text-muted-foreground">USD / {t("settings.deepseekMonth")}</span>
              <span className="text-xs text-muted-foreground">({t("settings.deepseekBudgetUnlimitedHint")})</span>
            </div>

            {period && (
              <div className="space-y-2 rounded-md border border-border p-3">
                <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
                  <span className="font-medium text-foreground">
                    <Coins className="mr-1 inline h-3.5 w-3.5 text-amber-500" />
                    {t("settings.deepseekPeriod", { start: period.periodStart || "—", end: period.periodEnd || "—" })}
                  </span>
                  <StatusPill tone={period.peakNow ? "warn" : "ok"}>
                    {period.peakNow ? t("settings.deepseekPeakNow") : t("settings.deepseekOffpeakNow")}
                  </StatusPill>
                </div>
                <div className="h-2 w-full rounded-full bg-muted">
                  <div
                    className={cn(
                      "h-2 rounded-full transition-all",
                      ratio >= 1 ? "bg-destructive" : ratio >= warnRatio ? "bg-amber-500" : "bg-primary",
                    )}
                    style={{ width: `${budget > 0 ? ratio * 100 : Math.min(100, spent > 0 ? 100 : 0)}%` }}
                  />
                </div>
                <div className="flex flex-wrap justify-between gap-2 text-xs text-muted-foreground">
                  <span>
                    {t("settings.deepseekSpent", { spent: formatUsd(spent), budget: budget > 0 ? formatUsd(budget) : "∞" })}
                  </span>
                  <span>
                    {t("settings.deepseekPeriodTokens", {
                      tokens: (period.usage.usage.promptTokens + period.usage.usage.completionTokens).toLocaleString(),
                      discussions: period.usage.discussions,
                    })}
                  </span>
                </div>
                {period.history.length > 0 && (
                  <div className="max-h-32 space-y-1 overflow-y-auto border-t border-border pt-2">
                    {period.history.map((entry, i) => (
                      <div key={i} className="flex flex-wrap justify-between gap-2 text-xs text-muted-foreground">
                        <span>{entry.periodStart} → {entry.periodEnd}</span>
                        <span>{formatUsd(entry.costUsd)} · {entry.discussions} {t("settings.deepseekDiscussions")}</span>
                      </div>
                    ))}
                  </div>
                )}
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="text-[11px] text-muted-foreground">{t("settings.deepseekPricingDate", { date: period.pricingDate })}</span>
                  <button onClick={handleResetPeriod} disabled={resetting} className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50">
                    {resetting ? <Loader2 className="h-3 w-3 animate-spin" /> : <RotateCcw className="h-3 w-3" />}
                    {t("settings.deepseekPeriodReset")}
                  </button>
                </div>
              </div>
            )}
          </Field>
        </>
      )}
    </Section>
  );
}
