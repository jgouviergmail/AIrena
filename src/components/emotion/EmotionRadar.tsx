import { useTranslation } from "react-i18next";
import type { EmotionalProfile } from "@/lib/types";

export const EMOTION_AXES: { key: keyof EmotionalProfile; hue: number }[] = [
  { key: "engagement", hue: 30 },
  { key: "accord", hue: 220 },
  { key: "confiance", hue: 140 },
  { key: "frustration", hue: 0 },
  { key: "curiosite", hue: 190 },
  { key: "enthousiasme", hue: 55 },
];

const SIZE = 150;
const CENTER = SIZE / 2;
const RADIUS = 52;
const RINGS = [25, 50, 75, 100];

function point(index: number, value: number): [number, number] {
  const angle = -Math.PI / 2 + (index * 2 * Math.PI) / EMOTION_AXES.length;
  const r = (RADIUS * value) / 100;
  return [CENTER + r * Math.cos(angle), CENTER + r * Math.sin(angle)];
}

/** Six-axis radar of an emotional profile; the polygon animates between updates. */
export function EmotionRadar({ emotions, previous }: { emotions: EmotionalProfile; previous?: EmotionalProfile }) {
  const { t } = useTranslation();
  const polygon = EMOTION_AXES.map(({ key }, i) => point(i, emotions[key]).join(",")).join(" ");
  const ghost = previous ? EMOTION_AXES.map(({ key }, i) => point(i, previous[key]).join(",")).join(" ") : null;

  return (
    <svg viewBox={`0 0 ${SIZE} ${SIZE}`} className="mx-auto h-auto w-full max-w-56" role="img" aria-label={t("emotions.radar.label")}>
      {RINGS.map((ring) => (
        <polygon
          key={ring}
          points={EMOTION_AXES.map((_, i) => point(i, ring).join(",")).join(" ")}
          className="fill-none stroke-border"
          strokeWidth={0.6}
        />
      ))}
      {EMOTION_AXES.map((_, i) => {
        const [x, y] = point(i, 100);
        return <line key={i} x1={CENTER} y1={CENTER} x2={x} y2={y} className="stroke-border" strokeWidth={0.6} />;
      })}
      {ghost && (
        <polygon points={ghost} className="fill-none stroke-muted-foreground/40" strokeWidth={1} strokeDasharray="2 2" />
      )}
      <polygon
        points={polygon}
        className="fill-primary/25 stroke-primary motion-safe:transition-all motion-safe:duration-500"
        strokeWidth={1.5}
        strokeLinejoin="round"
      />
      {EMOTION_AXES.map(({ key, hue }, i) => {
        const [x, y] = point(i, emotions[key]);
        const [lx, ly] = point(i, 122);
        return (
          <g key={key}>
            <circle cx={x} cy={y} r={2.2} style={{ fill: `oklch(0.65 0.18 ${hue})` }} className="motion-safe:transition-all motion-safe:duration-500" />
            <text
              x={lx}
              y={ly}
              textAnchor="middle"
              dominantBaseline="middle"
              className="fill-muted-foreground text-[7px]"
            >
              {t(`emotions.${key}`)} {emotions[key]}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
