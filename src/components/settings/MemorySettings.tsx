import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Brain, Eraser, Loader2 } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { logger } from "@/lib/logger";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { toast } from "@/stores/useToastStore";
import { buttonClass, Explainer, Section } from "./SettingsPrimitives";

/** Long memory of the personas: on / off, how many memories exist, forget everything. */
export function MemorySettings() {
  const { t } = useTranslation();
  const enabled = useSettingsStore((s) => s.settings.personaMemoryEnabled);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const [count, setCount] = useState<number | null>(null);
  const [forgetting, setForgetting] = useState(false);

  // The counter is informative: a failure leaves it blank rather than raising a toast on every visit
  const refresh = useCallback(async () => {
    try {
      setCount(await api.countPersonaMemories());
    } catch (e: unknown) {
      setCount(null);
      logger.warn("settings", `Cannot count persona memories: ${extractErrorMessage(e)}`);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const forget = async () => {
    if (!window.confirm(t("memory.forgetConfirm"))) return;
    setForgetting(true);
    try {
      await api.forgetPersonaMemories();
      toast.success(t("memory.forgotten"));
      await refresh();
    } catch (e: unknown) {
      toast.error(t("memory.forgetError"), extractErrorMessage(e));
    } finally {
      setForgetting(false);
    }
  };

  return (
    <Section title={t("memory.title")} icon={Brain}>
      <Explainer>{t("memory.desc")}</Explainer>
      <div className="flex items-center gap-3">
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          onClick={() => updateSettings({ personaMemoryEnabled: !enabled })}
          className={cn("relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors", enabled ? "bg-primary" : "bg-muted")}
        >
          <span className={cn("pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform", enabled ? "translate-x-4" : "translate-x-0")} />
        </button>
        <span className="text-sm text-muted-foreground">{enabled ? t("memory.enabled") : t("memory.disabled")}</span>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-xs text-muted-foreground">{count === null ? "…" : t("memory.count", { count })}</span>
        <button type="button" onClick={forget} disabled={forgetting || count === 0} className={buttonClass}>
          {forgetting ? <Loader2 className="h-4 w-4 animate-spin" /> : <Eraser className="h-4 w-4" />}
          {t("memory.forget")}
        </button>
      </div>
    </Section>
  );
}
