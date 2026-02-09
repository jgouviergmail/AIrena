import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmotionSparkline } from "./EmotionSparkline";
import type { EmotionalProfile } from "@/lib/types";

interface EmotionAxisSliderProps {
  axis: keyof EmotionalProfile;
  value: number;
  history: number[];
  hue: number;
  pulse?: boolean;
  onChange: (value: number) => void;
}

export function EmotionAxisSlider({
  axis,
  value,
  history,
  hue,
  pulse,
  onChange,
}: EmotionAxisSliderProps) {
  const { t } = useTranslation();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [displayValue, setDisplayValue] = useState(value);

  // Sync display value when prop changes (from store/backend)
  useEffect(() => {
    setDisplayValue(value);
  }, [value]);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = parseInt(e.target.value);
      setDisplayValue(v);
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => onChange(v), 300);
    },
    [onChange],
  );

  const color = `oklch(0.65 0.18 ${hue})`;

  return (
    <div className={`flex items-center gap-1.5 ${pulse ? "animate-pulse" : ""}`}>
      <span className="w-[80px] truncate text-[10px] text-muted-foreground">
        {t(`emotions.${axis}`)}
      </span>
      <div className="relative flex-1 h-1.5">
        {/* Visual track */}
        <div className="absolute inset-0 rounded-full bg-muted" />
        {/* Fill bar */}
        <div
          className="absolute inset-y-0 left-0 rounded-full transition-[width] duration-150"
          style={{ width: `${displayValue}%`, background: color }}
        />
        {/* Invisible interactive range input */}
        <input
          type="range"
          min={0}
          max={100}
          step={1}
          defaultValue={value}
          key={value}
          onChange={handleChange}
          className="absolute inset-0 w-full cursor-pointer opacity-0"
        />
      </div>
      <EmotionSparkline data={history} color={color} />
      <span className="w-[25px] text-right font-mono text-[10px] text-muted-foreground">
        {displayValue}
      </span>
    </div>
  );
}
