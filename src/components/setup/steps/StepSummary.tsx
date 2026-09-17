import { useTranslation } from "react-i18next";
import { getProfileEmoji } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { describeActiveModel, useSettingsStore } from "@/stores/useSettingsStore";
import { SummaryRow } from "./shared";

export function StepSummary() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const documentUpdateGranularity = useSetupStore((s) => s.documentUpdateGranularity);
  const webSearchPool = useSetupStore((s) => s.webSearchPool);
  const wikiSearchPool = useSetupStore((s) => s.wikiSearchPool);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const ragDocuments = useSetupStore((s) => s.ragDocuments);
  const documentInjectionMode = useSetupStore((s) => s.documentInjectionMode);
  const features = useSetupStore((s) => s.features);
  const settings = useSettingsStore((s) => s.settings);
  const speakerModels = [arbitre.model, ...gladiateurs.map((g) => g.model)];
  const mixedOllama = settings.llmProvider === "ollama" && describeActiveModel(settings, speakerModels).includes("mixte");

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-border bg-card p-4 space-y-4">
        {/* Provider */}
        <SummaryRow label={t("setup.summaryProvider")} value={describeActiveModel(settings, speakerModels)} />
        {mixedOllama && (
          <p className="text-xs text-amber-600 dark:text-amber-400">{t("setup.summaryMixedModelsVram")}</p>
        )}
        {settings.llmProvider === "deepseek" && (
          <SummaryRow label={t("setup.summaryReasoning")} value={t(`settings.reasoning_${settings.reasoningLevel}`)} />
        )}

        <hr className="border-border" />

        {/* Discussion settings */}
        <SummaryRow label={t("setup.summaryDiscussionMode")} value={t(`setup.mode_${discussionMode}`)} />
        <SummaryRow label={t("setup.summaryTopic")} value={topic} />
        <SummaryRow label={t("setup.summaryLanguage")} value={t(`languages.${discussionLanguage}`)} />
        <SummaryRow
          label={t("setup.summaryTurns")}
          value={maxTurns != null ? String(maxTurns) : t("setup.maxTurnsPlaceholder")}
        />

        <hr className="border-border" />

        {/* IArbitre settings */}
        <SummaryRow label={t("setup.summaryArbitre")} value={arbitre.name} />
        <SummaryRow
          label={t("setup.summaryTurnDist")}
          value={discussionMode === "userDriven"
            ? t("setup.userDrivenNoTurnDist")
            : discussionMode === "collaborativeFiction"
              ? t("setup.fictionNoTurnDist")
              : t(`setup.${arbitre.turnDistribution}`)}
        />
        {documentFormat !== "none" && (
          <>
            <SummaryRow label={t("setup.summaryDocFormat")} value={`.${documentFormat}`} />
            <SummaryRow label={t("setup.summaryDocGranularity")} value={t(`setup.docGranularity_${documentUpdateGranularity}`)} />
          </>
        )}
        {argumentMapEnabled && (
          <SummaryRow label={t("setup.summaryArgumentMap")} value={t("setup.switchYes")} />
        )}
        <SummaryRow
          label={t("setup.staging")}
          value={[
            t(`setup.reactionTiming_${features.reactionTiming}`),
            ...(["audienceReactions", "sceneEvents", "hiddenAgenda", "coalitions"] as const).filter((k) => features[k]).map((k) => t(`setup.feature_${k}`)),
          ].join(" · ")}
        />
        {(arbitre.webSearchIntro ?? false) && <SummaryRow label={t("setup.summaryWebIntro")} value="1" />}
        {(arbitre.wikiSearchIntro ?? false) && <SummaryRow label={t("setup.summaryWikiIntro")} value="1" />}

        <hr className="border-border" />

        {/* GladIAteurs */}
        <div>
          <p className="text-xs text-muted-foreground">{t("setup.summaryGladiateurs")}</p>
          {gladiateurs.length === 0 ? (
            <p className="mt-1 text-sm font-medium text-foreground">-</p>
          ) : (
            <div className="mt-1 space-y-1">
              {gladiateurs.map((g) => (
                <div key={g.id} className="flex items-center gap-2 text-sm text-foreground">
                  <span>{g.emoji ?? getProfileEmoji(g.name, g.systemPrompt)}</span>
                  <span className="font-medium">{g.name}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <SummaryRow
          label={t("setup.summaryEmotionDriven")}
          value={settings.emotionDriven ? t("setup.switchYes") : t("setup.switchNo")}
        />
        {webSearchPool > 0 && (
          <SummaryRow label={t("setup.summaryWebPool")} value={t("setup.webSearchBudget", { count: webSearchPool })} />
        )}
        {wikiSearchPool > 0 && (
          <SummaryRow label={t("setup.summaryWikiPool")} value={t("setup.wikiSearchBudget", { count: wikiSearchPool })} />
        )}
        {ragDocuments.length > 0 && (
          <>
            <div>
              <p className="text-xs text-muted-foreground">{t("setup.summaryRag")}</p>
              <div className="mt-1 space-y-0.5">
                {ragDocuments.map((doc) => (
                  <p key={doc.docId} className="text-sm font-medium text-foreground">
                    {doc.fileName} ({doc.chunkCount} {t("setup.ragChunks")})
                  </p>
                ))}
              </div>
            </div>
            <SummaryRow
              label={t("setup.summaryInjectionMode")}
              value={documentInjectionMode === "fullInjection" ? t("setup.injectionModeFull") : t("setup.injectionModeRag")}
            />
          </>
        )}
      </div>
    </div>
  );
}
