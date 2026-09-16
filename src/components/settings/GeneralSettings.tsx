import { useTranslation } from "react-i18next";
import { Globe, Moon, Settings as SettingsIcon, Sun } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import { ChoiceRow, Field, inputClass, Section } from "./SettingsPrimitives";

export function GeneralSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const usernameMissing = !settings.username.trim();

  return (
    <Section title={t("settings.general")} icon={SettingsIcon}>
      <Field label={t("settings.username")}>
        <input
          type="text"
          value={settings.username}
          onChange={(e) => updateSettings({ username: e.target.value })}
          placeholder={t("settings.usernamePlaceholder")}
          className={cn(inputClass, usernameMissing ? "border-destructive" : "border-input")}
        />
        {usernameMissing && <p className="mt-1 text-xs text-destructive">{t("settings.usernameRequired")}</p>}
      </Field>

      <Field label={t("settings.language")}>
        <ChoiceRow
          value={settings.language}
          onChange={(language) => updateSettings({ language })}
          options={(["fr", "en", "zh"] as const).map((lang) => ({
            value: lang,
            label: <><Globe className="h-3.5 w-3.5" />{t(`languages.${lang}`)}</>,
          }))}
        />
      </Field>

      <Field label={t("settings.theme")}>
        <ChoiceRow
          value={settings.theme}
          onChange={(theme) => updateSettings({ theme })}
          options={[
            { value: "dark", label: <><Moon className="h-3.5 w-3.5" />{t("settings.themeDark")}</> },
            { value: "light", label: <><Sun className="h-3.5 w-3.5" />{t("settings.themeLight")}</> },
          ]}
        />
      </Field>
    </Section>
  );
}
