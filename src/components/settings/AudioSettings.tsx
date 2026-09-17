import { useTranslation } from "react-i18next";
import { Volume2 } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { speech, sounds } from "@/hooks/useArenaAudio";
import { TTS_MODES } from "@/lib/types";
import { cn } from "@/lib/utils";
import { ChoiceRow, Explainer, Field, Section } from "./SettingsPrimitives";

function Switch({ checked, onChange, label }: { checked: boolean; onChange: () => void; label: string }) {
  return (
    <div className="flex items-center gap-3">
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={onChange}
        className={cn(
          "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
          checked ? "bg-primary" : "bg-muted",
        )}
      >
        <span className={cn("pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform", checked ? "translate-x-4" : "translate-x-0")} />
      </button>
      <span className="text-sm text-muted-foreground">{label}</span>
    </div>
  );
}

function Volume({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <label className="text-xs text-muted-foreground">{label}</label>
        <span className="font-mono text-xs text-foreground">{Math.round(value * 100)} %</span>
      </div>
      <input type="range" min={0} max={1} step={0.05} value={value} onChange={(e) => onChange(parseFloat(e.target.value))} className="w-full accent-primary" />
    </div>
  );
}

/** Voice (speech synthesis) and procedural sounds of the arena (v1.18). */
export function AudioSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  return (
    <Section title={t("settings.audio")} icon={Volume2}>
      <Field label={t("settings.tts")}>
        <Explainer>{t("settings.ttsDesc")}</Explainer>
        {!speech.available && <p className="text-xs text-amber-600">{t("settings.ttsUnavailable")}</p>}
        <Switch checked={settings.ttsEnabled} onChange={() => updateSettings({ ttsEnabled: !settings.ttsEnabled })} label={t("settings.ttsEnabled")} />
        {settings.ttsEnabled && (
          <>
            <ChoiceRow
              value={settings.ttsMode}
              onChange={(ttsMode) => updateSettings({ ttsMode })}
              options={TTS_MODES.map((mode) => ({ value: mode, label: t(`settings.ttsMode_${mode}`), title: t(`settings.ttsMode_${mode}Desc`) }))}
            />
            <p className="text-xs text-muted-foreground">{t(`settings.ttsMode_${settings.ttsMode}Desc`)}</p>
            <Volume value={settings.ttsVolume} onChange={(ttsVolume) => updateSettings({ ttsVolume })} label={t("settings.ttsVolume")} />
          </>
        )}
      </Field>

      <Field label={t("settings.sounds")}>
        <Explainer>{t("settings.soundsDesc")}</Explainer>
        {!sounds.available && <p className="text-xs text-amber-600">{t("settings.soundsUnavailable")}</p>}
        <Switch checked={settings.soundEnabled} onChange={() => updateSettings({ soundEnabled: !settings.soundEnabled })} label={t("settings.soundEnabled")} />
        {settings.soundEnabled && (
          <>
            <Volume value={settings.soundVolume} onChange={(soundVolume) => updateSettings({ soundVolume })} label={t("settings.soundVolume")} />
            <button
              type="button"
              onClick={() => {
                sounds.enabled = true;
                sounds.volume = settings.soundVolume;
                void sounds.unlock().then(() => sounds.play("gong"));
              }}
              className="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-accent"
            >
              {t("settings.soundTest")}
            </button>
          </>
        )}
      </Field>
    </Section>
  );
}
