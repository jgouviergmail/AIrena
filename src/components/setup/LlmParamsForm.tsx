import { useTranslation } from "react-i18next";
import type { LlmParams } from "@/lib/types";

interface Props {
  params: LlmParams;
  onChange: (patch: Partial<LlmParams>) => void;
}

export function LlmParamsForm({ params, onChange }: Props) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-2 gap-3">
      <SliderField
        label={t("setup.temperature")}
        value={params.temperature}
        min={0}
        max={2}
        step={0.1}
        onChange={(v) => onChange({ temperature: v })}
      />
      <SliderField
        label={t("setup.topP")}
        value={params.topP}
        min={0}
        max={1}
        step={0.05}
        onChange={(v) => onChange({ topP: v })}
      />
      <SliderField
        label={t("setup.topK")}
        value={params.topK}
        min={1}
        max={100}
        step={1}
        onChange={(v) => onChange({ topK: v })}
      />
      <NumberField
        label={t("setup.numPredict")}
        value={params.numPredict}
        min={64}
        max={4096}
        onChange={(v) => onChange({ numPredict: v })}
      />
      <NumberField
        label={t("setup.numCtx")}
        value={params.numCtx}
        min={2048}
        max={131072}
        onChange={(v) => onChange({ numCtx: v })}
      />
      <SliderField
        label={t("setup.repeatPenalty")}
        value={params.repeatPenalty}
        min={1}
        max={2}
        step={0.05}
        onChange={(v) => onChange({ repeatPenalty: v })}
      />
    </div>
  );
}

function SliderField({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <label className="text-xs text-muted-foreground">{label}</label>
        <span className="text-xs font-mono text-foreground">{value}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full accent-primary"
      />
    </div>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-muted-foreground">{label}</label>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(parseInt(e.target.value) || min)}
        className="w-full rounded-md border border-input bg-background px-2 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
      />
    </div>
  );
}
