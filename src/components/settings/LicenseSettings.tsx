import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff, KeyRound, Loader2, ShieldCheck, ShieldX } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import type { LicenseStatus } from "@/lib/types";
import * as api from "@/lib/tauri-api";
import { buttonClass, Field, secretInputClass, Section, StatusPill } from "./SettingsPrimitives";

export function LicenseSettings() {
  const { t } = useTranslation();
  const licenseKey = useSettingsStore((s) => s.settings.licenseKey);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const [showKey, setShowKey] = useState(false);
  const [status, setStatus] = useState<LicenseStatus | null>(null);
  const [validating, setValidating] = useState(false);

  useEffect(() => {
    api.checkLicenseStatus().then(setStatus).catch(() => {});
  }, []);

  const handleValidate = async () => {
    const key = licenseKey.trim();
    if (!key) return;
    setValidating(true);
    try {
      const next = await api.validateLicenseKey(key);
      setStatus(next);
      if (next.valid) {
        toast.success(t("settings.licenseValid"));
      } else {
        toast.error(t("settings.licenseInvalid"), next.error ?? "");
      }
    } catch (e: unknown) {
      toast.error(t("settings.licenseError"), extractErrorMessage(e));
    } finally {
      setValidating(false);
    }
  };

  return (
    <Section title={t("settings.license")} icon={KeyRound}>
      <Field label={t("settings.licenseKey")}>
        <div className="flex flex-wrap gap-2">
          <div className="relative min-w-48 flex-1">
            <input
              type={showKey ? "text" : "password"}
              value={licenseKey}
              onChange={(e) => updateSettings({ licenseKey: e.target.value })}
              placeholder="AIRENA-..."
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
          <button onClick={handleValidate} disabled={validating || !licenseKey.trim()} className={buttonClass}>
            {validating ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <KeyRound className="h-3.5 w-3.5" />}
            {t("settings.licenseValidate")}
          </button>
        </div>
      </Field>

      <Field label={t("settings.licenseStatus")}>
        {!licenseKey.trim() ? (
          <StatusPill tone="muted">{t("settings.licenseNone")}</StatusPill>
        ) : status?.valid ? (
          <div className="space-y-1">
            <StatusPill tone="ok" icon={<ShieldCheck className="h-3.5 w-3.5" />}>{t("settings.licenseActive")}</StatusPill>
            <p className="text-xs text-muted-foreground">
              {t("settings.licenseExpires", {
                date: new Date(status.expiresAt * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" }),
              })}
            </p>
          </div>
        ) : status ? (
          <div className="space-y-1">
            <StatusPill tone="error" icon={<ShieldX className="h-3.5 w-3.5" />}>
              {status.error === "License expired" ? t("settings.licenseExpired") : t("settings.licenseInvalid")}
            </StatusPill>
            {status.error && <p className="text-xs text-destructive">{status.error}</p>}
          </div>
        ) : null}
      </Field>
    </Section>
  );
}
