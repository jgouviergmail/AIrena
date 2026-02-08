import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  Check,
  Globe,
  Loader2,
  Moon,
  Pencil,
  Sun,
  Trash2,
  Wifi,
  WifiOff,
} from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useTheme } from "@/providers/ThemeProvider";
import { cn } from "@/lib/utils";
import type { PredefinedProfile } from "@/lib/types";

export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { setTheme } = useTheme();
  const settings = useSettingsStore((s) => s.settings);
  const profiles = useSettingsStore((s) => s.profiles);
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
  const [, setEditingProfile] = useState<PredefinedProfile | null>(null);

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
      <TopBar title={t("settings.title")} />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl space-y-8">
          {/* General */}
          <Section title={t("settings.general")}>
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
          <Section title={t("settings.ollama")}>
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

          {/* Profiles */}
          <Section title={t("settings.profiles")}>
            <div className="space-y-2">
              {profiles.map((p) => (
                <div
                  key={p.id}
                  className="flex items-center justify-between rounded-md border border-border bg-card p-3"
                >
                  <div>
                    <span className="text-sm font-medium text-foreground">
                      {t(`profiles.${p.id}.name`, { defaultValue: p.name })}
                    </span>
                    <span className="ml-2 text-xs text-muted-foreground">
                      {t(`profiles.${p.id}.personality`, { defaultValue: p.personality })}
                    </span>
                  </div>
                  <div className="flex gap-1">
                    {!p.isBuiltin && (
                      <>
                        <button
                          onClick={() => setEditingProfile(p)}
                          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                        >
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={() => useSettingsStore.getState().deleteProfile(p.id)}
                          className="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </Section>

          {/* Save button */}
          <div className="flex justify-end">
            <button
              onClick={handleSave}
              disabled={!settings.username.trim()}
              className={cn(
                "flex items-center gap-2 rounded-lg px-6 py-2.5 text-sm font-medium transition-colors disabled:opacity-50",
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
          </div>
        </div>
      </div>
    </>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <h2 className="mb-4 text-lg font-semibold text-foreground">{title}</h2>
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
