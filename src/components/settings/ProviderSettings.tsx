import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Brain, Cloud, Cpu, Eye } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import { REASONING_LEVELS } from "@/lib/types";
import type { ProviderKind } from "@/lib/types";
import { ChoiceRow, Explainer, Field, Section } from "./SettingsPrimitives";

const PROVIDERS: { value: ProviderKind; icon: typeof Cpu }[] = [
  { value: "ollama", icon: Cpu },
  { value: "deepseek", icon: Cloud },
];

/** Which backend serves discussions + the global reasoning policy. */
export function ProviderSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const setProvider = useSettingsStore((s) => s.setProvider);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const loadLlmConstants = useSettingsStore((s) => s.loadLlmConstants);

  // Bounds are needed to re-anchor numCtx when the provider changes
  useEffect(() => {
    loadLlmConstants();
  }, [loadLlmConstants]);

  return (
    <Section title={t("settings.provider")} icon={Cloud}>
      <Field label={t("settings.providerChoice")}>
        <Explainer>{t("settings.providerDesc")}</Explainer>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {PROVIDERS.map(({ value, icon: Icon }) => (
            <button
              key={value}
              type="button"
              onClick={() => setProvider(value)}
              className={cn(
                "flex items-start gap-3 rounded-md border px-3 py-2 text-left transition-colors",
                settings.llmProvider === value
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              <Icon className="mt-0.5 h-4 w-4 shrink-0" />
              <div>
                <div className="text-sm font-medium">{t(`settings.provider_${value}`)}</div>
                <div className="mt-0.5 text-xs opacity-70">{t(`settings.provider_${value}Desc`)}</div>
              </div>
            </button>
          ))}
        </div>
      </Field>

      {settings.llmProvider === "deepseek" && (
        <>
          <Field label={<><Brain className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.reasoningLevel")}</>}>
            <Explainer>{t("settings.reasoningLevelDesc")}</Explainer>
            <ChoiceRow
              value={settings.reasoningLevel}
              onChange={(reasoningLevel) => updateSettings({ reasoningLevel })}
              options={REASONING_LEVELS.map((level) => ({
                value: level,
                label: t(`settings.reasoning_${level}`),
                title: t(`settings.reasoning_${level}Desc`),
              }))}
            />
            <p className="text-xs text-muted-foreground">{t(`settings.reasoning_${settings.reasoningLevel}Desc`)}</p>
          </Field>

          <Field label={<><Eye className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.showModelReasoning")}</>}>
            <div className="flex items-center gap-3">
              <button
                type="button"
                role="switch"
                aria-checked={settings.showModelReasoning}
                onClick={() => updateSettings({ showModelReasoning: !settings.showModelReasoning })}
                className={cn(
                  "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                  settings.showModelReasoning ? "bg-primary" : "bg-muted",
                )}
              >
                <span
                  className={cn(
                    "pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
                    settings.showModelReasoning ? "translate-x-4" : "translate-x-0",
                  )}
                />
              </button>
              <span className="text-sm text-muted-foreground">{t("settings.showModelReasoningDesc")}</span>
            </div>
          </Field>
        </>
      )}
    </Section>
  );
}
