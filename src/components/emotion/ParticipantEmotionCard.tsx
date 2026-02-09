import { EmotionAxisSlider } from "./EmotionAxisSlider";
import type { EmotionalProfile, EmotionSnapshot } from "@/lib/types";

const AXES: { key: keyof EmotionalProfile; hue: number }[] = [
  { key: "engagement", hue: 30 },
  { key: "accord", hue: 220 },
  { key: "confiance", hue: 140 },
  { key: "frustration", hue: 0 },
  { key: "curiosite", hue: 190 },
  { key: "enthousiasme", hue: 55 },
];

interface ParticipantEmotionCardProps {
  participant: { id: string; name: string; emoji: string; role: string };
  emotions: EmotionalProfile;
  history: EmotionSnapshot[];
  onAdjust: (axis: string, value: number) => void;
  thresholdAxis?: string;
  moodSummary?: string;
  currentTurn: number;
}

export function ParticipantEmotionCard({
  participant,
  emotions,
  history,
  onAdjust,
  thresholdAxis,
  moodSummary,
  currentTurn,
}: ParticipantEmotionCardProps) {
  return (
    <div className="space-y-1 rounded-md border border-border bg-card p-2">
      <div className="flex items-center gap-1.5">
        <span className="text-sm">{participant.emoji}</span>
        <span className="truncate text-xs font-medium text-foreground">
          {participant.name}
        </span>
        {participant.role === "IArbitre" && (
          <span className="rounded bg-primary/10 px-1 text-[9px] font-medium text-primary">
            IArbitre
          </span>
        )}
      </div>
      {moodSummary && currentTurn >= 2 && (
        <p className="truncate text-[10px] font-bold italic text-muted-foreground">{moodSummary}</p>
      )}
      {AXES.map(({ key, hue }) => (
        <EmotionAxisSlider
          key={key}
          axis={key}
          value={emotions[key]}
          history={history.map((s) => s.emotions[key])}
          hue={hue}
          pulse={thresholdAxis === key}
          onChange={(v) => onAdjust(key, v)}
        />
      ))}
    </div>
  );
}
