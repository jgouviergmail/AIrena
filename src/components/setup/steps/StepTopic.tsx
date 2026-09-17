import { useTranslation } from "react-i18next";
import { Clock, FileText, Globe, MessageSquare, Network, Repeat } from "lucide-react";
import { useSetupStore } from "@/stores/useSetupStore";
import { cn } from "@/lib/utils";
import { DISCUSSION_MODES, DOCUMENT_FORMATS, inputClass, OptionCard, SectionLabel, Toggle } from "./shared";
import { LivelinessOptions } from "./LivelinessOptions";
import { TemplatePicker } from "@/components/setup/TemplatePicker";

export function StepTopic() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const setTopic = useSetupStore((s) => s.setTopic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const setDiscussionLanguage = useSetupStore((s) => s.setDiscussionLanguage);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const setDiscussionMode = useSetupStore((s) => s.setDiscussionMode);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const setDocumentFormat = useSetupStore((s) => s.setDocumentFormat);
  const documentUpdateGranularity = useSetupStore((s) => s.documentUpdateGranularity);
  const setDocumentUpdateGranularity = useSetupStore((s) => s.setDocumentUpdateGranularity);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const setArgumentMapEnabled = useSetupStore((s) => s.setArgumentMapEnabled);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const setMaxTurns = useSetupStore((s) => s.setMaxTurns);
  const userInterventionTimeoutSecs = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const setUserTimeout = useSetupStore((s) => s.setUserTimeout);

  return (
    <div className="space-y-6">
      <TemplatePicker />

      <div className="space-y-2">
        <SectionLabel icon={Globe}>{t("setup.discussionLanguage")}</SectionLabel>
        <div className="flex flex-wrap gap-2">
          {(["fr", "en", "zh"] as const).map((lang) => (
            <button
              key={lang}
              onClick={() => setDiscussionLanguage(lang)}
              className={cn(
                "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
                discussionLanguage === lang
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              <Globe className="h-3.5 w-3.5" />
              {t(`languages.${lang}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        <SectionLabel icon={Repeat}>{t("setup.maxTurns")}</SectionLabel>
        <input
          type="number"
          min={1}
          max={100}
          value={maxTurns ?? ""}
          onChange={(e) => setMaxTurns(e.target.value ? parseInt(e.target.value) : null)}
          placeholder={t("setup.maxTurnsPlaceholder")}
          className={inputClass}
        />
      </div>

      {/* Discussion mode selector */}
      <div className="space-y-2">
        <SectionLabel icon={MessageSquare}>{t("setup.discussionMode")}</SectionLabel>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4">
          {DISCUSSION_MODES.map((mode) => (
            <OptionCard
              key={mode}
              selected={discussionMode === mode}
              title={t(`setup.mode_${mode}`)}
              description={t(`setup.mode_${mode}Desc`)}
              onClick={() => setDiscussionMode(mode)}
            />
          ))}
        </div>
      </div>

      <div className="space-y-2">
        <SectionLabel icon={MessageSquare}>{t("setup.topic")}</SectionLabel>
        <textarea
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder={t("setup.topicPlaceholder")}
          rows={4}
          className={inputClass}
        />
      </div>

      {/* Argument Map toggle */}
      <div className="space-y-2">
        <SectionLabel icon={Network}>{t("setup.argumentMap")}</SectionLabel>
        <p className="text-xs text-muted-foreground">{t("setup.argumentMapDesc")}</p>
        <Toggle
          checked={argumentMapEnabled}
          onChange={setArgumentMapEnabled}
          caption={argumentMapEnabled ? t("setup.switchYes") : t("setup.switchNo")}
        />
      </div>

      <LivelinessOptions />

      {/* Document format selector */}
      <div className="space-y-2">
        <SectionLabel icon={FileText}>{t("setup.documentFormat")}</SectionLabel>
        <p className="text-xs text-muted-foreground">{t("setup.documentFormatDesc")}</p>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {DOCUMENT_FORMATS.map((fmt) => (
            <button
              key={fmt}
              onClick={() => setDocumentFormat(fmt)}
              className={cn(
                "rounded-md border px-3 py-2 text-center text-sm transition-colors",
                documentFormat === fmt
                  ? "border-primary bg-primary/10 text-primary font-medium"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              {t(`setup.docFormat_${fmt}`)}
            </button>
          ))}
        </div>
        {documentFormat !== "none" && (
          <div className="space-y-1.5 pt-1">
            <p className="text-xs text-muted-foreground">{t("setup.docGranularityDesc")}</p>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
              <OptionCard
                selected={documentUpdateGranularity === "turn"}
                title={t("setup.docGranularity_turn")}
                description={t("setup.docGranularity_turnDesc")}
                onClick={() => setDocumentUpdateGranularity("turn")}
              />
              <OptionCard
                selected={documentUpdateGranularity === "intervention"}
                title={t("setup.docGranularity_intervention")}
                description={t("setup.docGranularity_interventionDesc")}
                onClick={() => setDocumentUpdateGranularity("intervention")}
              />
            </div>
          </div>
        )}
      </div>

      <div className="space-y-1.5">
        <SectionLabel icon={Clock}>{t("setup.userTimeout")}</SectionLabel>
        <input
          type="number"
          min={30}
          max={600}
          value={userInterventionTimeoutSecs}
          onChange={(e) => setUserTimeout(parseInt(e.target.value) || 120)}
          className={inputClass}
        />
      </div>
    </div>
  );
}
