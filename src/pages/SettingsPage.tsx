import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Loader2 } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { GeneralSettings } from "@/components/settings/GeneralSettings";
import { LicenseSettings } from "@/components/settings/LicenseSettings";
import { ProviderSettings } from "@/components/settings/ProviderSettings";
import { DeepSeekSettings } from "@/components/settings/DeepSeekSettings";
import { OllamaSettings } from "@/components/settings/OllamaSettings";
import { TokenBudgetPrioritiesSettings } from "@/components/settings/TokenBudgetPriorities";
import { TavilySettings } from "@/components/settings/TavilySettings";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useTheme } from "@/providers/ThemeProvider";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";

/** Debounce before persisting a settings change (ms). */
const AUTOSAVE_DELAY_MS = 800;
const SAVED_BADGE_MS = 2000;

/**
 * Settings page: one section per concern. Every change is auto-saved
 * (debounced) except the DeepSeek API key, which its section validates first.
 */
export default function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { setTheme } = useTheme();
  const settings = useSettingsStore((s) => s.settings);
  const loading = useSettingsStore((s) => s.loading);
  const hydrate = useSettingsStore((s) => s.hydrate);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [autoSaved, setAutoSaved] = useState(false);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

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
        setTimeout(() => setAutoSaved(false), SAVED_BADGE_MS);
      } catch (e) {
        toast.error(t("settings.saveError"), extractErrorMessage(e));
      }
    }, AUTOSAVE_DELAY_MS);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, loading]);

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

  const isDeepSeek = settings.llmProvider === "deepseek";

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
      <div className="flex-1 overflow-y-auto p-4 sm:p-6">
        <div className="mx-auto max-w-2xl space-y-8">
          <GeneralSettings />
          <LicenseSettings />
          <ProviderSettings />
          {isDeepSeek && <DeepSeekSettings />}
          <OllamaSettings chatEnabled={!isDeepSeek} />
          <TokenBudgetPrioritiesSettings />
          <TavilySettings />
        </div>
      </div>
    </>
  );
}
