import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { EmotionalProfile } from "@/lib/types";

const AXES: (keyof EmotionalProfile)[] = [
  "engagement",
  "accord",
  "confiance",
  "frustration",
  "curiosite",
  "enthousiasme",
];

const AXIS_COLORS: Record<string, string> = {
  engagement: "bg-orange-500",
  accord: "bg-blue-500",
  confiance: "bg-green-500",
  frustration: "bg-red-500",
  curiosite: "bg-cyan-500",
  enthousiasme: "bg-yellow-500",
};

function getDominantColor(emotions: EmotionalProfile): string {
  let max = 0;
  let dominant = "engagement";
  for (const axis of AXES) {
    if (emotions[axis] > max) {
      max = emotions[axis];
      dominant = axis;
    }
  }
  const hues: Record<string, number> = {
    engagement: 30,
    accord: 220,
    confiance: 140,
    frustration: 0,
    curiosite: 190,
    enthousiasme: 55,
  };
  return `oklch(0.65 0.18 ${hues[dominant]})`;
}

export function EmotionIndicator({
  emotions,
}: {
  emotions: EmotionalProfile;
}) {
  const { t } = useTranslation();
  const [showTooltip, setShowTooltip] = useState(false);

  return (
    <div
      className="relative"
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
    >
      <div
        className="h-3 w-3 rounded-full transition-colors duration-500"
        style={{ backgroundColor: getDominantColor(emotions) }}
      />

      {showTooltip && (
        <div className="absolute bottom-full left-1/2 z-50 mb-2 -translate-x-1/2 rounded-lg border border-border bg-popover p-3 shadow-lg">
          <div className="space-y-1.5" style={{ minWidth: 160 }}>
            {AXES.map((axis) => (
              <div key={axis} className="flex items-center gap-2">
                <span className="w-20 text-[10px] text-muted-foreground">
                  {t(`emotions.${axis}`)}
                </span>
                <div className="h-1.5 flex-1 rounded-full bg-muted">
                  <div
                    className={`h-full rounded-full transition-all duration-500 ${AXIS_COLORS[axis]}`}
                    style={{ width: `${emotions[axis]}%` }}
                  />
                </div>
                <span className="w-6 text-right text-[10px] font-mono text-muted-foreground">
                  {emotions[axis]}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
