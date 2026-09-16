import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Brain, ChevronDown, ChevronUp, Crosshair } from "lucide-react";
import { EmotionAxisSlider } from "./EmotionAxisSlider";
import { EMOTION_AXES, EmotionRadar } from "./EmotionRadar";
import { cn } from "@/lib/utils";
import type { BanInfo, DirectiveData, EmotionalProfile, EmotionSnapshot } from "@/lib/types";

export type EmotionView = "bars" | "radar";

/** Map an emotional profile to a face emoji illustrating the dominant state. */
function getEmotionEmoji(emotions: EmotionalProfile): string {
  if (emotions.frustration >= 80) return "😤";
  if (emotions.frustration >= 65) return "😠";
  if (emotions.enthousiasme >= 80) return "🤩";
  if (emotions.enthousiasme >= 65) return "😃";
  if (emotions.confiance >= 80) return "😎";
  if (emotions.confiance <= 20) return "😰";
  if (emotions.curiosite >= 75) return "🧐";
  if (emotions.engagement <= 25) return "😶";
  if (emotions.engagement >= 80) return "😊";
  if (emotions.accord <= 25) return "😒";
  if (emotions.accord >= 80) return "😌";
  if (emotions.frustration <= 15) return "😌";
  return "🙂";
}

const AXES = EMOTION_AXES;

interface ParticipantEmotionCardProps {
  participant: { id: string; name: string; emoji: string; role: string };
  emotions: EmotionalProfile;
  history: EmotionSnapshot[];
  onAdjust: (axis: string, value: number) => void;
  thresholdAxis?: string;
  moodSummary?: string;
  currentTurn: number;
  directive?: DirectiveData;
  banInfo?: BanInfo;
  view?: EmotionView;
  isActive?: boolean;
}

export function ParticipantEmotionCard({
  participant,
  emotions,
  history,
  onAdjust,
  thresholdAxis,
  moodSummary,
  currentTurn,
  directive,
  banInfo,
  view = "bars",
  isActive = false,
}: ParticipantEmotionCardProps) {
  const { t } = useTranslation();
  const [showDirective, setShowDirective] = useState(false);
  const previous = history.length >= 2 ? history[history.length - 2].emotions : undefined;

  return (
    <div
      className={cn(
        "space-y-1 rounded-md border bg-card p-2 transition-colors",
        isActive ? "border-primary/60 shadow-[0_0_0_1px] shadow-primary/20" : "border-border",
      )}
    >
      <div className="flex items-center gap-1.5">
        <span className="text-sm">{participant.emoji}</span>
        <span className={cn(
          "truncate text-xs font-medium",
          banInfo && banInfo.remaining > 0 ? "text-destructive" : "text-foreground",
        )}>
          {participant.name}
        </span>
        {participant.role === "IArbitre" && (
          <span className="rounded bg-primary/10 px-1 text-[9px] font-medium text-primary">
            IArbitre
          </span>
        )}
      </div>
      {banInfo && banInfo.remaining > 0 && (
        <div className="flex items-center gap-1 text-[10px] font-medium text-destructive">
          <span>{t("emotions.ban.remaining", { count: banInfo.remaining })}</span>
        </div>
      )}
      {banInfo && banInfo.totalBans > 0 && banInfo.remaining === 0 && (
        <span className="text-[10px] text-muted-foreground">
          {t("emotions.ban.totalCount", { count: banInfo.totalBans })}
        </span>
      )}
      {moodSummary && currentTurn >= 2 && (
        <p className="truncate text-[10px] font-bold italic text-muted-foreground">
          <span className="not-italic">{getEmotionEmoji(emotions)}</span>{" "}
          {moodSummary}
        </p>
      )}
      {view === "radar" ? (
        <EmotionRadar emotions={emotions} previous={previous} />
      ) : (
        AXES.map(({ key, hue }) => (
          <EmotionAxisSlider
            key={key}
            axis={key}
            value={emotions[key]}
            history={history.map((s) => s.emotions[key])}
            hue={hue}
            pulse={thresholdAxis === key}
            onChange={(v) => onAdjust(key, v)}
          />
        ))
      )}

      {directive && (
        <div className="border-t border-border pt-1">
          <button
            onClick={() => setShowDirective(!showDirective)}
            className="flex w-full items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground"
          >
            <span>🎭</span>
            <span className="flex-1 text-left font-medium">
              {t("directive.backstage")}
            </span>
            {showDirective ? (
              <ChevronUp className="h-3 w-3" />
            ) : (
              <ChevronDown className="h-3 w-3" />
            )}
          </button>
          {showDirective && (
            <div className="mt-1 space-y-1 text-[10px] text-muted-foreground">
              <div>
                <span className="font-medium text-foreground">
                  {t("directive.speechAct")}:
                </span>{" "}
                <span className="rounded bg-accent px-1 py-0.5">
                  {t(`directive.acts.${directive.speechAct}`, directive.speechAct)}
                </span>
              </div>
              <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
                <span className="inline-flex items-center gap-0.5">
                  <Crosshair className="h-3 w-3" />
                  <span className="font-medium text-foreground">{t("directive.focus")}:</span>{" "}
                  {directive.focusSpeaker ?? t("directive.focusTopic")}
                </span>
                {directive.reasoningLevel !== "off" && (
                  <span className="inline-flex items-center gap-0.5">
                    <Brain className="h-3 w-3" />
                    {t(`settings.reasoning_${directive.reasoningLevel}`)}
                  </span>
                )}
              </div>
              {directive.emotionBehavior && (
                <div>
                  <span className="font-medium text-foreground">
                    {t("directive.behavior")}:
                  </span>{" "}
                  {directive.emotionBehavior}
                </div>
              )}
              {directive.relationshipSummary && (
                <div>
                  <span className="font-medium text-foreground">
                    {t("directive.relationships")}:
                  </span>{" "}
                  {directive.relationshipSummary.split(", ").map((entry, i) => {
                    const sep = entry.lastIndexOf(": ");
                    if (sep === -1) return <span key={i}>{i > 0 && ", "}{entry}</span>;
                    const name = entry.slice(0, sep);
                    const kind = entry.slice(sep + 2);
                    return (
                      <span key={i}>
                        {i > 0 && ", "}
                        {name}: {t(`directive.relations.${kind}`, kind)}
                      </span>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
