import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ExternalLink, FileDown, Info, Loader2 } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { logger } from "@/lib/logger";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { toast } from "@/stores/useToastStore";
import { buttonClass, Explainer, Section } from "./SettingsPrimitives";

/** Version, update check (releases page) and the journal export (frontend buffer + backend log tail). */
export function AboutSettings() {
  const { t } = useTranslation();
  const llmConstants = useSettingsStore((s) => s.llmConstants);
  const loadLlmConstants = useSettingsStore((s) => s.loadLlmConstants);
  const [version, setVersion] = useState<string>("");
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    loadLlmConstants();
    api.getAppVersion().then(setVersion).catch(() => setVersion(""));
  }, [loadLlmConstants]);

  const exportJournal = async () => {
    setExporting(true);
    try {
      const backend = await api.readBackendLog();
      const stamp = new Date().toISOString();
      const content = [
        `# AIrena ${version} — ${t("about.journalTitle")} (${stamp})`,
        "",
        `## ${t("about.journalBackend")}`,
        backend.trim() || t("about.journalEmpty"),
        "",
        `## ${t("about.journalFrontend")}`,
        logger.export(),
        "",
      ].join("\n");
      await api.downloadTextFile(content, "AIrena - journal.txt");
    } catch (e: unknown) {
      toast.error(t("about.exportError"), extractErrorMessage(e));
    } finally {
      setExporting(false);
    }
  };

  const releases = llmConstants?.releasesUrl;

  return (
    <Section title={t("about.title")} icon={Info}>
      <p className="text-sm text-foreground">{t("about.version", { version: version || "—" })}</p>
      <Explainer>{t("about.updatesDesc")}</Explainer>
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" onClick={() => releases && api.openExternalUrl(releases)} disabled={!releases} className={buttonClass}>
          <ExternalLink className="h-4 w-4" />
          {t("about.checkUpdates")}
        </button>
        <button type="button" onClick={exportJournal} disabled={exporting} className={buttonClass}>
          {exporting ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileDown className="h-4 w-4" />}
          {t("about.exportJournal")}
        </button>
      </div>
      <p className="text-xs text-muted-foreground">{t("about.journalDesc")}</p>
    </Section>
  );
}
