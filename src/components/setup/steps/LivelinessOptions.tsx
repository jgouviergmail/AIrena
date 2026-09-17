import { useTranslation } from "react-i18next";
import { Sparkles } from "lucide-react";
import { useSetupStore } from "@/stores/useSetupStore";
import { modeSupportsHiddenAgenda } from "@/lib/modes";
import type { DiscussionFeatures, ReactionTiming } from "@/lib/types";
import { OptionCard, SectionLabel, Toggle } from "./shared";

const REACTION_TIMINGS: ReactionTiming[] = ["immediate", "deferred"];

/** Boolean liveliness features, in display order (each has i18n keys `setup.feature_<key>` / `…Desc`). */
const TOGGLES: (keyof Omit<DiscussionFeatures, "reactionTiming">)[] = ["audienceReactions", "sceneEvents", "hiddenAgenda", "coalitions"];

/** "Mise en scène" block of the first wizard step: how alive the discussion is. */
export function LivelinessOptions() {
  const { t } = useTranslation();
  const features = useSetupStore((s) => s.features);
  const setFeatures = useSetupStore((s) => s.setFeatures);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const speakers = Math.max(gladiateurs.length, 2);
  // Immediate reactions cost (N−1) short calls per intervention: say it plainly
  const extraCalls = speakers - 1;

  return (
    <div className="space-y-3">
      <SectionLabel icon={Sparkles}>{t("setup.staging")}</SectionLabel>
      <p className="text-xs text-muted-foreground">{t("setup.stagingDesc")}</p>

      <div className="space-y-1.5">
        <p className="text-xs font-medium text-foreground">{t("setup.reactionTiming")}</p>
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {REACTION_TIMINGS.map((timing) => (
            <OptionCard
              key={timing}
              selected={features.reactionTiming === timing}
              title={t(`setup.reactionTiming_${timing}`)}
              description={timing === "immediate" ? t("setup.reactionTiming_immediateDesc", { count: extraCalls }) : t("setup.reactionTiming_deferredDesc")}
              onClick={() => setFeatures({ reactionTiming: timing })}
            />
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {TOGGLES.map((key) => (
          <div key={key} className="space-y-1 rounded-md border border-border px-3 py-2">
            <p className="text-xs font-medium text-foreground">{t(`setup.feature_${key}`)}</p>
            <p className="text-[11px] text-muted-foreground">{t(`setup.feature_${key}Desc`)}</p>
            {key === "hiddenAgenda" && features.hiddenAgenda && !modeSupportsHiddenAgenda(discussionMode) && (
              <p className="text-[11px] text-amber-600 dark:text-amber-400">{t("setup.feature_hiddenAgendaNotInMode")}</p>
            )}
            {key === "hiddenAgenda" && !features.hiddenAgenda && discussionMode === "negotiation" && (
              <p className="text-[11px] text-amber-600 dark:text-amber-400">{t("setup.feature_hiddenAgendaForced")}</p>
            )}
            <Toggle
              checked={features[key]}
              onChange={(next) => setFeatures({ [key]: next })}
              caption={features[key] ? t("setup.switchYes") : t("setup.switchNo")}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
