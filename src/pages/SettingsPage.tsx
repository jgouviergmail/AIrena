import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  Check,
  Eye,
  EyeOff,
  Globe,
  Loader2,
  Moon,
  Search,
  Server,
  Settings as SettingsIcon,
  Sun,
  Wifi,
  WifiOff,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useTheme } from "@/providers/ThemeProvider";
import { cn } from "@/lib/utils";
import type { TavilyPeriodHistory } from "@/lib/types";

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

  const [saved, setSaved] = useState(false);
  const [checking, setChecking] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  useEffect(() => {
    checkOllama();
  }, [checkOllama]);

  const handleSave = async () => {
    try {
      await saveSettings();
      i18n.changeLanguage(settings.language);
      setTheme(settings.theme as "dark" | "light");
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
  };

  const handleCheckOllama = async () => {
    setChecking(true);
    try {
      await checkOllama();
    } finally {
      setChecking(false);
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
        <button
          onClick={handleSave}
          disabled={!settings.username.trim()}
          className={cn(
            "flex items-center gap-2 rounded-lg px-5 py-2 text-sm font-medium transition-colors disabled:opacity-50",
            saved
              ? "bg-green-500/10 text-green-500"
              : "bg-primary text-primary-foreground hover:bg-primary/90",
          )}
        >
          {saved ? (
            <>
              <Check className="h-4 w-4" />
              {t("settings.saved")}
            </>
          ) : (
            t("settings.save")
          )}
        </button>
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
            <Field label={t("settings.emotionDriven")}>
              <div className="flex items-center gap-3">
                <button
                  onClick={() =>
                    updateSettings({ emotionDriven: !settings.emotionDriven })
                  }
                  className={cn(
                    "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                    settings.emotionDriven ? "bg-primary" : "bg-muted",
                  )}
                >
                  <span
                    className={cn(
                      "pointer-events-none inline-block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform",
                      settings.emotionDriven
                        ? "translate-x-5"
                        : "translate-x-0",
                    )}
                  />
                </button>
                <span className="text-sm text-muted-foreground">
                  {t("settings.emotionDrivenDesc")}
                </span>
              </div>
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
                <select
                  value={settings.ollamaModel}
                  onChange={(e) => {
                    const model = e.target.value;
                    updateSettings({ ollamaModel: model });
                    if (model) {
                      preloadModel(model);
                    }
                  }}
                  disabled={preloading}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
                >
                  <option value="">--</option>
                  {models.map((m) => (
                    <option key={m.name} value={m.name}>
                      {m.name} ({(m.size / 1e9).toFixed(1)} GB)
                    </option>
                  ))}
                </select>
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
  label: string;
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
