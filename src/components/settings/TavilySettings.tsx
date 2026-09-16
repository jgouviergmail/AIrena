import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff, Globe, Search } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import type { TavilyPeriodHistory } from "@/lib/types";
import { Field, inputClass, Section, secretInputClass } from "./SettingsPrimitives";

/** Tavily free tier: monthly credits. */
const TAVILY_FREE_CREDITS = 1000;
const TAVILY_WARN_CREDITS = 500;
const TAVILY_CRITICAL_CREDITS = 800;

export function TavilySettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const [showApiKey, setShowApiKey] = useState(false);

  let history: TavilyPeriodHistory[] = [];
  try {
    history = JSON.parse(settings.tavilyUsageHistory || "[]");
  } catch { /* ignore */ }
  const hasCurrentPeriod = !!settings.tavilyPeriodStart && settings.tavilyUsageCount > 0;

  return (
    <Section title={t("settings.tavily")} icon={Globe}>
      <Field label={t("settings.tavilyApiKey")}>
        <div className="relative">
          <input
            type={showApiKey ? "text" : "password"}
            value={settings.tavilyApiKey}
            onChange={(e) => updateSettings({ tavilyApiKey: e.target.value })}
            placeholder="tvly-..."
            className={secretInputClass}
          />
          <button
            type="button"
            onClick={() => setShowApiKey(!showApiKey)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
          </button>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          <a href="https://www.tavily.com" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
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
              onChange={(e) => updateSettings({ tavilyPeriodStart: e.target.value })}
              className={`${inputClass} w-auto`}
            />
          </Field>

          <div className="space-y-2">
            <div className="flex flex-wrap items-center gap-3">
              <label className="whitespace-nowrap text-sm font-medium text-muted-foreground">{t("settings.tavilyUsageCount")}</label>
              <input
                type="number"
                min={0}
                max={TAVILY_FREE_CREDITS}
                value={settings.tavilyUsageCount}
                onChange={(e) =>
                  updateSettings({
                    tavilyUsageCount: Math.max(0, Math.min(TAVILY_FREE_CREDITS, parseInt(e.target.value) || 0)),
                  })
                }
                className={`${inputClass} w-24`}
              />
              <span className="whitespace-nowrap text-sm text-muted-foreground">
                / {TAVILY_FREE_CREDITS} ({t("settings.tavilyFree")})
              </span>
            </div>
            <div className="h-2 w-full rounded-full bg-muted">
              <div
                className={cn(
                  "h-2 rounded-full transition-all",
                  settings.tavilyUsageCount > TAVILY_CRITICAL_CREDITS
                    ? "bg-destructive"
                    : settings.tavilyUsageCount > TAVILY_WARN_CREDITS
                      ? "bg-yellow-500"
                      : "bg-primary",
                )}
                style={{ width: `${Math.min(100, (settings.tavilyUsageCount / TAVILY_FREE_CREDITS) * 100)}%` }}
              />
            </div>
          </div>

          <Field label={t("settings.tavilyHistory")}>
            {history.length === 0 && !hasCurrentPeriod ? (
              <p className="text-sm text-muted-foreground">{t("settings.tavilyNoHistory")}</p>
            ) : (
              <div className="max-h-48 space-y-1 overflow-y-auto rounded-md border border-border p-2">
                {hasCurrentPeriod && (
                  <div className="flex items-center gap-2 text-xs font-medium text-foreground">
                    <Search className="h-3 w-3 shrink-0 text-primary" />
                    {t("settings.tavilyCurrentPeriod", { start: settings.tavilyPeriodStart, count: settings.tavilyUsageCount })}
                  </div>
                )}
                {history.map((entry, i) => (
                  <div key={i} className="flex items-center gap-2 text-xs text-muted-foreground">
                    <Search className="h-3 w-3 shrink-0" />
                    {t("settings.tavilyPeriodEntry", { start: entry.periodStart, end: entry.periodEnd, count: entry.usageCount })}
                  </div>
                ))}
              </div>
            )}
          </Field>
        </>
      )}
    </Section>
  );
}
