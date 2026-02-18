import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  ArrowDown,
  ArrowUp,
  Check,
  Database,
  Eye,
  EyeOff,
  Globe,
  KeyRound,
  Layers,
  Loader2,
  Moon,
  RotateCcw,
  Search,
  Server,
  Settings as SettingsIcon,
  ShieldCheck,
  ShieldX,
  Sun,
  Wifi,
  WifiOff,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { VramIndicator } from "@/components/setup/VramIndicator";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useTheme } from "@/providers/ThemeProvider";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import { CONFIGURABLE_BUDGET_SECTIONS } from "@/lib/types";
import type { LicenseStatus, SectionPriority, TavilyPeriodHistory } from "@/lib/types";
import * as api from "@/lib/tauri-api";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { setTheme } = useTheme();
  const settings = useSettingsStore((s) => s.settings);
  const models = useSettingsStore((s) => s.models);
  const ollamaConnected = useSettingsStore((s) => s.ollamaConnected);
  const loading = useSettingsStore((s) => s.loading);
  const hydrate = useSettingsStore((s) => s.hydrate);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const checkOllama = useSettingsStore((s) => s.checkOllama);
  const preloadModel = useSettingsStore((s) => s.preloadModel);
  const preloading = useSettingsStore((s) => s.preloading);
  const preloadDone = useSettingsStore((s) => s.preloadDone);
  const preloadError = useSettingsStore((s) => s.preloadError);
  const modelBudgetInfo = useSettingsStore((s) => s.modelBudgetInfo);
  const modelBudgetLoading = useSettingsStore((s) => s.modelBudgetLoading);
  const fetchModelBudgetInfo = useSettingsStore((s) => s.fetchModelBudgetInfo);
  const initializingOllama = useSettingsStore((s) => s.initializingOllama);
  const ollamaInitialized = useSettingsStore((s) => s.ollamaInitialized);

  const [autoSaved, setAutoSaved] = useState(false);
  const [checking, setChecking] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [showLicenseKey, setShowLicenseKey] = useState(false);
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus | null>(null);
  const [validatingLicense, setValidatingLicense] = useState(false);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  // Skip Ollama check if initialization is already in progress
  useEffect(() => {
    if (!initializingOllama) {
      checkOllama();
    }
  }, [checkOllama, initializingOllama]);

  useEffect(() => {
    api.checkLicenseStatus().then(setLicenseStatus).catch(() => {});
  }, []);

  // Track whether model was changed by user (vs hydration/mount)
  const userChangedModelRef = useRef(false);

  // Fetch model budget info (VRAM + architecture) when model changes
  // Skip if initialization is in progress (it already fetches budget info)
  useEffect(() => {
    if (settings.ollamaModel && !initializingOllama) {
      fetchModelBudgetInfo(settings.ollamaModel, userChangedModelRef.current);
      userChangedModelRef.current = false;
    }
  }, [settings.ollamaModel, fetchModelBudgetInfo, initializingOllama]);

  // numCtx local state — commit on blur/Enter to avoid clamping on every keystroke
  const [numCtxInput, setNumCtxInput] = useState(String(settings.numCtx));
  useEffect(() => {
    setNumCtxInput(String(settings.numCtx));
  }, [settings.numCtx]);

  const numCtxMax = modelBudgetInfo?.recommendedNumCtx
    ? Math.round(modelBudgetInfo.recommendedNumCtx * 1.5)
    : 131072;

  const commitNumCtx = () => {
    const parsed = parseInt(numCtxInput) || 2048;
    const clamped = Math.max(2048, Math.min(numCtxMax, parsed));
    setNumCtxInput(String(clamped));
    const prev = settings.numCtx;
    updateSettings({ numCtx: clamped });
    // Reload the model with the new num_ctx so Ollama reallocates the KV cache.
    // Without this, changing num_ctx only takes effect at next discussion start.
    if (clamped !== prev && settings.ollamaModel) {
      preloadModel(settings.ollamaModel);
    }
  };

  // Auto-save settings on change (debounced, skip initial hydration)
  const saveTimerRef = useRef<ReturnType<typeof setTimeout>>(null);
  const hydratedRef = useRef(false);

  useEffect(() => {
    if (loading) return;
    if (!hydratedRef.current) {
      hydratedRef.current = true;
      return;
    }
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(async () => {
      try {
        await saveSettings();
        i18n.changeLanguage(settings.language);
        setTheme(settings.theme as "dark" | "light");
        setAutoSaved(true);
        setTimeout(() => setAutoSaved(false), 2000);
      } catch (e) {
        toast.error(t("settings.saveError"), extractErrorMessage(e));
      }
    }, 800);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, loading]);

  const handleCheckOllama = async () => {
    setChecking(true);
    try {
      await checkOllama();
    } finally {
      setChecking(false);
    }
  };

  const handleValidateLicense = async () => {
    const key = settings.licenseKey.trim();
    if (!key) return;
    setValidatingLicense(true);
    try {
      const status = await api.validateLicenseKey(key);
      setLicenseStatus(status);
      if (status.valid) {
        toast.success(t("settings.licenseValid"));
      } else {
        toast.error(t("settings.licenseInvalid"), status.error ?? "");
      }
    } catch (e: unknown) {
      toast.error(t("settings.licenseError"), extractErrorMessage(e));
    } finally {
      setValidatingLicense(false);
    }
  };

  const handleUrlChange = async (url: string) => {
    updateSettings({ ollamaUrl: url });
  };

  if (loading) {
    return (
      <>
        <TopBar title={t("settings.title")} />
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      </>
    );
  }

  return (
    <>
      <TopBar title={t("settings.title")}>
        {autoSaved && (
          <span className="flex items-center gap-1.5 text-sm text-green-500 animate-in fade-in">
            <Check className="h-4 w-4" />
            {t("settings.saved")}
          </span>
        )}
      </TopBar>
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl space-y-8">
          {/* General */}
          <Section title={t("settings.general")} icon={SettingsIcon}>
            <Field label={t("settings.username")}>
              <input
                type="text"
                value={settings.username}
                onChange={(e) => updateSettings({ username: e.target.value })}
                placeholder={t("settings.usernamePlaceholder")}
                className={cn(
                  "w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring",
                  !settings.username.trim()
                    ? "border-destructive"
                    : "border-input",
                )}
              />
              {!settings.username.trim() && (
                <p className="mt-1 text-xs text-destructive">
                  {t("settings.usernameRequired")}
                </p>
              )}
            </Field>

            <Field label={t("settings.language")}>
              <div className="flex gap-2">
                {(["fr", "en", "zh"] as const).map((lang) => (
                  <button
                    key={lang}
                    onClick={() => updateSettings({ language: lang })}
                    className={cn(
                      "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
                      settings.language === lang
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-border text-muted-foreground hover:bg-accent",
                    )}
                  >
                    <Globe className="h-3.5 w-3.5" />
                    {t(`languages.${lang}`)}
                  </button>
                ))}
              </div>
            </Field>

            <Field label={t("settings.theme")}>
              <div className="flex gap-2">
                {(
                  [
                    { value: "dark", icon: Moon, labelKey: "settings.themeDark" },
                    { value: "light", icon: Sun, labelKey: "settings.themeLight" },
                  ] as const
                ).map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => updateSettings({ theme: opt.value })}
                    className={cn(
                      "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
                      settings.theme === opt.value
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-border text-muted-foreground hover:bg-accent",
                    )}
                  >
                    <opt.icon className="h-3.5 w-3.5" />
                    {t(opt.labelKey)}
                  </button>
                ))}
              </div>
            </Field>
          </Section>

          {/* License */}
          <Section title={t("settings.license")} icon={KeyRound}>
            <Field label={t("settings.licenseKey")}>
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <input
                    type={showLicenseKey ? "text" : "password"}
                    value={settings.licenseKey}
                    onChange={(e) =>
                      updateSettings({ licenseKey: e.target.value })
                    }
                    placeholder="AIRENA-..."
                    className="w-full rounded-md border border-input bg-background px-3 py-2 pr-9 text-sm text-foreground font-mono focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                  <button
                    type="button"
                    onClick={() => setShowLicenseKey(!showLicenseKey)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showLicenseKey ? (
                      <EyeOff className="h-4 w-4" />
                    ) : (
                      <Eye className="h-4 w-4" />
                    )}
                  </button>
                </div>
                <button
                  onClick={handleValidateLicense}
                  disabled={validatingLicense || !settings.licenseKey.trim()}
                  className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-50"
                >
                  {validatingLicense ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <KeyRound className="h-3.5 w-3.5" />
                  )}
                  {t("settings.licenseValidate")}
                </button>
              </div>
            </Field>

            <Field label={t("settings.licenseStatus")}>
              {!settings.licenseKey.trim() ? (
                <div className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium bg-muted text-muted-foreground">
                  <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground" />
                  {t("settings.licenseNone")}
                </div>
              ) : licenseStatus?.valid ? (
                <div className="space-y-1">
                  <div className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium bg-green-500/10 text-green-500">
                    <ShieldCheck className="h-3.5 w-3.5" />
                    {t("settings.licenseActive")}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {t("settings.licenseExpires", {
                      date: new Date(
                        licenseStatus.expiresAt * 1000,
                      ).toLocaleString(undefined, {
                        dateStyle: "medium",
                        timeStyle: "short",
                      }),
                    })}
                  </p>
                </div>
              ) : licenseStatus ? (
                <div className="space-y-1">
                  <div className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium bg-destructive/10 text-destructive">
                    <ShieldX className="h-3.5 w-3.5" />
                    {licenseStatus.error === "License expired"
                      ? t("settings.licenseExpired")
                      : t("settings.licenseInvalid")}
                  </div>
                  {licenseStatus.error && (
                    <p className="text-xs text-destructive">
                      {licenseStatus.error}
                    </p>
                  )}
                </div>
              ) : null}
            </Field>
          </Section>

          {/* Ollama */}
          <Section title={t("settings.ollama")} icon={Server}>
            <Field label={t("settings.ollamaUrl")}>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={settings.ollamaUrl}
                  onChange={(e) => handleUrlChange(e.target.value)}
                  className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                />
                <button
                  onClick={handleCheckOllama}
                  disabled={checking}
                  className="flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-50"
                >
                  {checking ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : ollamaConnected ? (
                    <Wifi className="h-3.5 w-3.5 text-green-500" />
                  ) : (
                    <WifiOff className="h-3.5 w-3.5 text-destructive" />
                  )}
                  {t("settings.ollamaCheck")}
                </button>
              </div>
            </Field>

            <Field label={t("settings.ollamaStatus")}>
              <div
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium",
                  ollamaConnected
                    ? "bg-green-500/10 text-green-500"
                    : "bg-destructive/10 text-destructive",
                )}
              >
                <span
                  className={cn(
                    "h-1.5 w-1.5 rounded-full",
                    ollamaConnected ? "bg-green-500" : "bg-destructive",
                  )}
                />
                {ollamaConnected
                  ? t("settings.ollamaConnected")
                  : t("settings.ollamaDisconnected")}
              </div>
            </Field>

            {ollamaConnected && models.length > 0 && (
              <Field label={t("settings.ollamaModel")}>
                <div className="flex items-center gap-2">
                  <Server className="h-4 w-4 text-primary" />
                  <select
                    value={settings.ollamaModel}
                    onChange={(e) => {
                      const model = e.target.value;
                      userChangedModelRef.current = true;
                      updateSettings({ ollamaModel: model });
                      if (model) {
                        preloadModel(model);
                      }
                    }}
                    disabled={preloading}
                    className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                  >
                    <option value="">--</option>
                    {models.map((m) => (
                      <option key={m.name} value={m.name}>
                        {m.name} ({(m.size / 1e9).toFixed(1)} GB)
                      </option>
                    ))}
                  </select>
                </div>
                {preloading && (
                  <div className="mt-1.5 flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    {t("settings.modelPreloading")}
                  </div>
                )}
                {preloadDone && !preloading && (
                  <div className="mt-1.5 flex items-center gap-1.5 text-xs text-green-500">
                    <Check className="h-3 w-3" />
                    {t("settings.modelPreloaded")}
                  </div>
                )}
                {preloadError && !preloading && (
                  <div className="mt-1.5 flex items-center gap-1.5 text-xs text-destructive">
                    <AlertCircle className="h-3 w-3" />
                    {t("settings.modelPreloadError")}
                  </div>
                )}
              </Field>
            )}

            {ollamaConnected && models.length > 0 && (
              <Field label={t("settings.embeddingModel")}>
                <div className="flex items-center gap-2">
                  <Database className="h-4 w-4 text-purple-500" />
                  <select
                    value={settings.embeddingModel}
                    onChange={async (e) => {
                      const model = e.target.value;
                      updateSettings({ embeddingModel: model });
                      try {
                        // Force save so backend reads the new embedding model, then refresh VRAM info
                        await saveSettings();
                        if (settings.ollamaModel) {
                          fetchModelBudgetInfo(settings.ollamaModel, false);
                        }
                      } catch (err: unknown) {
                        toast.error(t("settings.saveError"), extractErrorMessage(err));
                      }
                    }}
                    className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                  >
                    <option value="">{t("settings.embeddingModelAuto")}</option>
                    {/* Embedding-oriented models first */}
                    {models
                      .filter((m) => /embed|nomic/i.test(m.name))
                      .map((m) => (
                        <option key={m.name} value={m.name}>
                          {m.name} ({(m.size / 1e9).toFixed(1)} GB)
                        </option>
                      ))}
                    {/* Separator if there are embedding models */}
                    {models.some((m) => /embed|nomic/i.test(m.name)) && (
                      <option disabled>───</option>
                    )}
                    {/* Other models */}
                    {models
                      .filter((m) => !/embed|nomic/i.test(m.name))
                      .map((m) => (
                        <option key={m.name} value={m.name}>
                          {m.name} ({(m.size / 1e9).toFixed(1)} GB)
                        </option>
                      ))}
                  </select>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  {t("settings.embeddingModelDesc")}
                </p>
              </Field>
            )}

            {/* Ollama initialization indicator */}
            {initializingOllama && !ollamaInitialized && (
              <div className="flex items-center gap-2 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-xs text-primary">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                {t("settings.ollamaInitializing")}
              </div>
            )}

            {/* Context window size (numCtx) + VRAM indicator */}
            {ollamaConnected && settings.ollamaModel && (
              <Field label={<><Layers className="inline h-3.5 w-3.5 mr-1" />{t("settings.numCtxLabel")}</>}>
                {/* 1. Explanatory text (directly under title) */}
                <div className="rounded-md border border-border/50 bg-muted/30 px-3 py-2">
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {t("settings.numCtxDesc")}
                  </p>
                </div>

                {/* 2. VRAM indicator (GPU info + refresh) */}
                <VramIndicator
                  info={modelBudgetInfo}
                  loading={modelBudgetLoading || initializingOllama}
                  onRefresh={async () => {
                    if (settings.ollamaModel) {
                      try {
                        await saveSettings();
                        fetchModelBudgetInfo(settings.ollamaModel, false);
                      } catch (err: unknown) {
                        toast.error(t("settings.saveError"), extractErrorMessage(err));
                      }
                    }
                  }}
                />

                {/* 3. numCtx input + AUTO button */}
                <div className="flex items-center gap-3">
                  <input
                    type="number"
                    min={2048}
                    max={numCtxMax}
                    value={numCtxInput}
                    onChange={(e) => setNumCtxInput(e.target.value)}
                    onBlur={commitNumCtx}
                    onKeyDown={(e) => e.key === "Enter" && commitNumCtx()}
                    className="w-32 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground font-mono focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                  <span className="text-sm text-muted-foreground">tokens</span>
                  {modelBudgetInfo?.recommendedNumCtx && (
                    <button
                      onClick={() => {
                        const rec = modelBudgetInfo.recommendedNumCtx!;
                        updateSettings({ numCtx: rec });
                        if (rec !== settings.numCtx && settings.ollamaModel) {
                          preloadModel(settings.ollamaModel);
                        }
                      }}
                      title={t("setup.vramAutoFillTooltip", { value: modelBudgetInfo.recommendedNumCtx.toLocaleString() })}
                      className={cn(
                        "flex items-center gap-1 rounded-md border border-primary/30 bg-primary/5 px-2 py-1 text-xs font-medium text-primary",
                        "transition-colors hover:bg-primary/10",
                      )}
                    >
                      <Zap className="h-3 w-3" />
                      AUTO — {modelBudgetInfo.recommendedNumCtx.toLocaleString()}
                    </button>
                  )}
                </div>
              </Field>
            )}

            {!ollamaConnected && (
              <div className="rounded-lg border border-border bg-muted/50 p-4">
                <h4 className="mb-1 text-sm font-medium text-foreground">
                  {t("settings.ollamaGuide")}
                </h4>
                <p className="text-sm text-muted-foreground">
                  {t("settings.ollamaGuideText")}
                </p>
                <ol className="mt-2 list-inside list-decimal space-y-1 text-sm text-muted-foreground">
                  <li>
                    {t("settings.ollamaGuideStep1")}{" "}
                    <span className="font-mono text-primary">
                      https://ollama.com
                    </span>
                  </li>
                  <li>{t("settings.ollamaGuideStep2")}</li>
                  <li>
                    {t("settings.ollamaGuideStep3")}{" "}
                    <span className="font-mono text-primary">
                      ollama pull llama3.2
                    </span>
                  </li>
                  <li>{t("settings.ollamaGuideStep4")}</li>
                </ol>
              </div>
            )}
          </Section>

          {/* Tavily / Web Search */}
          <Section title={t("settings.tavily")} icon={Globe}>
            <Field label={t("settings.tavilyApiKey")}>
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <input
                    type={showApiKey ? "text" : "password"}
                    value={settings.tavilyApiKey}
                    onChange={(e) =>
                      updateSettings({ tavilyApiKey: e.target.value })
                    }
                    placeholder="tvly-..."
                    className="w-full rounded-md border border-input bg-background px-3 py-2 pr-9 text-sm text-foreground font-mono focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                  <button
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  >
                    {showApiKey ? (
                      <EyeOff className="h-4 w-4" />
                    ) : (
                      <Eye className="h-4 w-4" />
                    )}
                  </button>
                </div>
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                <a
                  href="https://www.tavily.com"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline"
                >
                  {t("settings.tavilyLink")}
                </a>
                {" — "}
                {t("settings.tavilyFreeCredits")}
              </p>
            </Field>

            {settings.tavilyApiKey.trim() && (
              <>
                <Field label={t("settings.tavilyPeriodStart")}>
                  <input
                    type="date"
                    value={settings.tavilyPeriodStart}
                    onChange={(e) =>
                      updateSettings({ tavilyPeriodStart: e.target.value })
                    }
                    className="rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                  />
                </Field>

                <Field label={t("settings.tavilyUsageCount")}>
                  <div className="space-y-2">
                    <div className="flex items-center gap-3">
                      <input
                        type="number"
                        min={0}
                        max={1000}
                        value={settings.tavilyUsageCount}
                        onChange={(e) =>
                          updateSettings({
                            tavilyUsageCount: Math.max(
                              0,
                              Math.min(1000, parseInt(e.target.value) || 0),
                            ),
                          })
                        }
                        className="w-24 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                      />
                      <span className="text-sm text-muted-foreground">
                        / 1000 ({t("settings.tavilyFree")})
                      </span>
                    </div>
                    <div className="h-2 w-full rounded-full bg-muted">
                      <div
                        className={cn(
                          "h-2 rounded-full transition-all",
                          settings.tavilyUsageCount > 800
                            ? "bg-destructive"
                            : settings.tavilyUsageCount > 500
                              ? "bg-yellow-500"
                              : "bg-primary",
                        )}
                        style={{
                          width: `${Math.min(100, (settings.tavilyUsageCount / 1000) * 100)}%`,
                        }}
                      />
                    </div>
                  </div>
                </Field>

                <Field label={t("settings.tavilyHistory")}>
                  {(() => {
                    let history: TavilyPeriodHistory[] = [];
                    try {
                      history = JSON.parse(
                        settings.tavilyUsageHistory || "[]",
                      );
                    } catch {
                      /* ignore */
                    }
                    const hasCurrentPeriod = settings.tavilyPeriodStart && settings.tavilyUsageCount > 0;
                    if (history.length === 0 && !hasCurrentPeriod) {
                      return (
                        <p className="text-sm text-muted-foreground">
                          {t("settings.tavilyNoHistory")}
                        </p>
                      );
                    }
                    return (
                      <div className="max-h-48 overflow-y-auto space-y-1 rounded-md border border-border p-2">
                        {hasCurrentPeriod && (
                          <div className="flex items-center gap-2 text-xs font-medium text-foreground">
                            <Search className="h-3 w-3 shrink-0 text-primary" />
                            {t("settings.tavilyCurrentPeriod", {
                              start: settings.tavilyPeriodStart,
                              count: settings.tavilyUsageCount,
                            })}
                          </div>
                        )}
                        {history.map((entry, i) => (
                          <div
                            key={i}
                            className="flex items-center gap-2 text-xs text-muted-foreground"
                          >
                            <Search className="h-3 w-3 shrink-0" />
                            {t("settings.tavilyPeriodEntry", {
                              start: entry.periodStart,
                              end: entry.periodEnd,
                              count: entry.usageCount,
                            })}
                          </div>
                        ))}
                      </div>
                    );
                  })()}
                </Field>
              </>
            )}
          </Section>

          {/* Token Budget Priorities */}
          <Section title={t("settings.tokenBudget")} icon={Layers}>
            <p className="text-sm text-muted-foreground">
              {t("settings.tokenBudgetDesc")}
            </p>
            <TokenBudgetPriorities
              value={settings.tokenBudgetPriorities}
              onChange={(json) => updateSettings({ tokenBudgetPriorities: json })}
            />
          </Section>
        </div>
      </div>
    </>
  );
}

function Section({
  title,
  icon: Icon,
  children,
}: {
  title: string;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border bg-card/50 p-5">
      <h2 className="mb-3 flex items-center gap-2 border-b border-border pb-3 text-lg font-semibold text-foreground">
        {Icon && <Icon className="h-5 w-5 text-primary" />}
        {title}
      </h2>
      <div className="space-y-4">{children}</div>
    </section>
  );
}

function Field({
  label,
  children,
}: {
  label: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-muted-foreground">
        {label}
      </label>
      {children}
    </div>
  );
}

function TokenBudgetPriorities({
  value,
  onChange,
}: {
  value: string;
  onChange: (json: string) => void;
}) {
  const { t } = useTranslation();

  // Parse stored priorities or build defaults
  const sections = (() => {
    try {
      if (value) {
        const parsed: SectionPriority[] = JSON.parse(value);
        // Filter to CONFIGURABLE sections only (document sections are auto-managed by backend)
        const configurable = parsed.filter((p) => CONFIGURABLE_BUDGET_SECTIONS.includes(p.section));
        const present = new Set(configurable.map((p) => p.section));
        if (CONFIGURABLE_BUDGET_SECTIONS.every((s) => present.has(s))) {
          return [...configurable].sort((a, b) => a.rank - b.rank);
        }
      }
    } catch { /* fall through to defaults */ }
    // Default order (rank 4-10)
    return CONFIGURABLE_BUDGET_SECTIONS.map((section, i) => ({
      section,
      rank: i + 4,
      floor: 0,
      ceiling: 0,
    }));
  })();

  const moveUp = (index: number) => {
    if (index <= 0) return;
    const updated = [...sections];
    [updated[index - 1], updated[index]] = [updated[index], updated[index - 1]];
    // Re-assign ranks
    const reranked = updated.map((s, i) => ({ ...s, rank: i + 4 }));
    onChange(JSON.stringify(reranked));
  };

  const moveDown = (index: number) => {
    if (index >= sections.length - 1) return;
    const updated = [...sections];
    [updated[index], updated[index + 1]] = [updated[index + 1], updated[index]];
    const reranked = updated.map((s, i) => ({ ...s, rank: i + 4 }));
    onChange(JSON.stringify(reranked));
  };

  const handleReset = () => {
    onChange("");
  };

  return (
    <div className="space-y-2">
      <p className="text-xs text-muted-foreground">{t("settings.tokenBudgetDragHint")}</p>
      <div className="space-y-1">
        {sections.map((s, i) => (
          <div
            key={s.section}
            className="flex items-center gap-2 rounded-md border border-border bg-background px-3 py-1.5"
          >
            <span className="w-5 text-center text-xs font-mono text-muted-foreground">
              {s.rank}
            </span>
            <span className="flex-1 text-sm text-foreground">
              {t(`setup.budgetSection_${s.section}`)}
            </span>
            <button
              onClick={() => moveUp(i)}
              disabled={i === 0}
              className="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-20"
            >
              <ArrowUp className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={() => moveDown(i)}
              disabled={i === sections.length - 1}
              className="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-20"
            >
              <ArrowDown className="h-3.5 w-3.5" />
            </button>
          </div>
        ))}
      </div>
      <button
        onClick={handleReset}
        className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
      >
        <RotateCcw className="h-3 w-3" />
        {t("settings.tokenBudgetReset")}
      </button>
    </div>
  );
}
