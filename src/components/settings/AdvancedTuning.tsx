import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronUp, RotateCcw, SlidersHorizontal } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { effectiveTuning, parseTuningOverrides, serialiseTuningOverrides, sliderStep, TUNING_GROUPS, type TuningInfo, type TuningKey } from "@/lib/tuning";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { toast } from "@/stores/useToastStore";
import { buttonClass, Explainer, Section } from "./SettingsPrimitives";

/**
 * "Réglages avancés" (collapsed): bounded sliders over the engine's dynamics.
 * Only the knobs that differ from the defaults are persisted; the backend
 * clamps whatever it reads, so a stale or hand-edited value never breaks a run.
 */
export function AdvancedTuning() {
  const { t } = useTranslation();
  const json = useSettingsStore((s) => s.settings.advancedTuningJson);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const [open, setOpen] = useState(false);
  const [info, setInfo] = useState<TuningInfo | null>(null);

  useEffect(() => {
    if (!open || info) return;
    api.getTuningInfo().then(setInfo).catch((e: unknown) => toast.error(t("tuning.loadError"), extractErrorMessage(e)));
  }, [open, info, t]);

  const values = useMemo(() => (info ? effectiveTuning(info.defaults, parseTuningOverrides(json)) : null), [info, json]);
  const overridden = useMemo(() => Object.keys(parseTuningOverrides(json)).length, [json]);

  const setKnob = (key: TuningKey, value: number) => {
    if (!info || !values) return;
    updateSettings({ advancedTuningJson: serialiseTuningOverrides({ ...values, [key]: value }, info.defaults) });
  };

  return (
    <Section title={t("tuning.title")} icon={SlidersHorizontal}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className="flex w-full items-center justify-between rounded-md border border-border px-3 py-2 text-left text-sm text-foreground hover:bg-accent"
      >
        <span>{overridden > 0 ? t("tuning.overridden", { count: overridden }) : t("tuning.allDefaults")}</span>
        {open ? <ChevronUp className="h-4 w-4 text-muted-foreground" /> : <ChevronDown className="h-4 w-4 text-muted-foreground" />}
      </button>
      {open && (
        <div className="space-y-4">
          <Explainer>{t("tuning.desc")}</Explainer>
          {!info || !values ? (
            <p className="text-sm text-muted-foreground">…</p>
          ) : (
            <>
              {TUNING_GROUPS.map((group) => (
                <div key={group.key} className="space-y-2">
                  <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{t(`tuning.group_${group.key}`)}</p>
                  {group.knobs.map((key) => {
                    const bounds = info.bounds[key];
                    const changed = values[key] !== info.defaults[key];
                    return (
                      <div key={key} className="space-y-0.5">
                        <div className="flex items-center justify-between gap-2">
                          <label htmlFor={`tuning-${key}`} className="text-xs text-foreground">{t(`tuning.knob_${key}`)}</label>
                          <span className={changed ? "font-mono text-xs text-primary" : "font-mono text-xs text-muted-foreground"}>
                            {values[key]}{changed ? ` (${info.defaults[key]})` : ""}
                          </span>
                        </div>
                        <input
                          id={`tuning-${key}`}
                          type="range"
                          min={bounds.min}
                          max={bounds.max}
                          step={sliderStep(bounds)}
                          value={values[key]}
                          onChange={(e) => setKnob(key, parseFloat(e.target.value))}
                          className="w-full accent-primary"
                        />
                        <p className="text-[11px] text-muted-foreground">{t(`tuning.knob_${key}Desc`)}</p>
                      </div>
                    );
                  })}
                </div>
              ))}
              <button type="button" onClick={() => updateSettings({ advancedTuningJson: "{}" })} disabled={overridden === 0} className={buttonClass}>
                <RotateCcw className="h-4 w-4" />
                {t("tuning.reset")}
              </button>
            </>
          )}
        </div>
      )}
    </Section>
  );
}
