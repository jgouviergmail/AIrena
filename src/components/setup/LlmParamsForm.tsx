import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Brain, Cpu, Info } from "lucide-react";
import { availableModels, globalModel, useSettingsStore } from "@/stores/useSettingsStore";
import { REASONING_LEVELS } from "@/lib/types";
import type { LlmParams, ReasoningLevel } from "@/lib/types";

interface Props {
  params: LlmParams;
  onChange: (patch: Partial<LlmParams>) => void;
  /** Model override of this speaker (undefined = the global model, v1.20) */
  model?: string;
  onModelChange?: (model: string | undefined) => void;
}

/** Local (Ollama) bound on generated tokens. */
const OLLAMA_MAX_NUM_PREDICT = 4096;
const NUM_PREDICT_MIN = 64;

/**
 * Sampling parameters of one speaker. The form follows the active provider:
 * Ollama exposes every knob; DeepSeek hides what its API ignores (top_k,
 * repeat_penalty), adds the reasoning level and reflects the API contract
 * (temperature ignored while thinking, top_p floored at 0.95).
 */
export function LlmParamsForm({ params, onChange, model, onModelChange }: Props) {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const ollamaModels = useSettingsStore((s) => s.models);
  const provider = settings.llmProvider;
  const globalReasoning = useSettingsStore((s) => s.settings.reasoningLevel);
  const llmConstants = useSettingsStore((s) => s.llmConstants);
  const loadLlmConstants = useSettingsStore((s) => s.loadLlmConstants);
  const isDeepSeek = provider === "deepseek";

  useEffect(() => {
    if (isDeepSeek && !llmConstants) loadLlmConstants();
  }, [isDeepSeek, llmConstants, loadLlmConstants]);

  const effectiveReasoning: ReasoningLevel = params.reasoningLevel ?? globalReasoning;
  const thinkingMayApply = isDeepSeek && effectiveReasoning !== "off";
  const maxNumPredict = isDeepSeek ? (llmConstants?.deepseekMaxNumPredictUi ?? OLLAMA_MAX_NUM_PREDICT) : OLLAMA_MAX_NUM_PREDICT;
  const topPMin = isDeepSeek && thinkingMayApply ? (llmConstants?.deepseekTopPMinThinking ?? 0.95) : 0;

  const models = onModelChange ? availableModels(settings, ollamaModels, llmConstants) : [];
  const inherited = globalModel(settings);

  return (
    <div className="space-y-3">
      {onModelChange && (
        <div className="space-y-1">
          <label className="flex items-center gap-1 text-xs text-muted-foreground">
            <Cpu className="h-3 w-3 text-primary" />
            {t("setup.speakerModel")}
          </label>
          <select
            value={model ?? ""}
            onChange={(e) => onModelChange(e.target.value === "" ? undefined : e.target.value)}
            className="w-full rounded-md border border-input bg-background px-2 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="">{t("setup.speakerModelInherit", { model: inherited || "—" })}</option>
            {models.filter((m) => m !== inherited).map((m) => <option key={m} value={m}>{m}</option>)}
            {model && !models.includes(model) && <option value={model}>{model}</option>}
          </select>
          {model && provider === "ollama" && <p className="text-[11px] text-muted-foreground">{t("setup.speakerModelVramHint")}</p>}
        </div>
      )}
      {isDeepSeek && (
        <div className="space-y-1">
          <label className="flex items-center gap-1 text-xs text-muted-foreground">
            <Brain className="h-3 w-3 text-primary" />
            {t("setup.reasoningLevel")}
          </label>
          <select
            value={params.reasoningLevel ?? ""}
            onChange={(e) => onChange({ reasoningLevel: e.target.value === "" ? undefined : (e.target.value as ReasoningLevel) })}
            className="w-full rounded-md border border-input bg-background px-2 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option value="">{t("setup.reasoningInherit", { level: t(`settings.reasoning_${globalReasoning}`) })}</option>
            {REASONING_LEVELS.map((level) => (
              <option key={level} value={level}>{t(`settings.reasoning_${level}`)}</option>
            ))}
          </select>
        </div>
      )}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <SliderField
          label={t("setup.temperature")}
          value={params.temperature}
          min={0}
          max={2}
          step={0.1}
          disabled={thinkingMayApply}
          hint={thinkingMayApply ? t("setup.temperatureIgnoredThinking") : undefined}
          onChange={(v) => onChange({ temperature: v })}
        />
        <SliderField
          label={t("setup.topP")}
          value={Math.max(params.topP, topPMin)}
          min={0}
          max={1}
          step={0.05}
          hint={topPMin > 0 ? t("setup.topPFloorThinking", { min: topPMin }) : undefined}
          onChange={(v) => onChange({ topP: Math.max(v, topPMin) })}
        />
        {!isDeepSeek && (
          <SliderField
            label={t("setup.topK")}
            value={params.topK}
            min={1}
            max={100}
            step={1}
            onChange={(v) => onChange({ topK: v })}
          />
        )}
        <NumberField
          label={t("setup.numPredict")}
          value={Math.min(params.numPredict, maxNumPredict)}
          min={NUM_PREDICT_MIN}
          max={maxNumPredict}
          onChange={(v) => onChange({ numPredict: Math.min(v, maxNumPredict) })}
        />
        {!isDeepSeek && (
          <SliderField
            label={t("setup.repeatPenalty")}
            value={params.repeatPenalty}
            min={1}
            max={2}
            step={0.05}
            onChange={(v) => onChange({ repeatPenalty: v })}
          />
        )}
      </div>
      {isDeepSeek && (
        <p className="flex items-start gap-1.5 text-xs text-muted-foreground">
          <Info className="mt-0.5 h-3 w-3 shrink-0 text-primary" />
          {t("setup.deepseekParamsNote")}
        </p>
      )}
    </div>
  );
}

function SliderField({
  label,
  value,
  min,
  max,
  step,
  disabled,
  hint,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  hint?: string;
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
        disabled={disabled}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full accent-primary disabled:opacity-40"
      />
      {hint && <p className="text-[11px] leading-snug text-muted-foreground/80">{hint}</p>}
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
