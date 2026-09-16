import { useTranslation } from "react-i18next";
import { Coins, Cpu } from "lucide-react";
import { formatUsd } from "@/lib/cost-estimate";
import type { LlmUsage } from "@/lib/types";

interface Props {
  provider: string;
  model?: string;
  total: LlmUsage;
  calls: number;
  /** null: free provider or unknown price list */
  estimatedCostUsd: number | null;
}

/** Token consumption and estimated cost of a discussion (summary + history). */
export function UsageSummaryCard({ provider, model, total, calls, estimatedCostUsd }: Props) {
  const { t } = useTranslation();
  const tokens = total.promptTokens + total.completionTokens;
  if (tokens === 0 && calls === 0) return null;
  const cachedPct = total.promptTokens > 0 ? Math.round((total.cachedTokens / total.promptTokens) * 100) : 0;
  const isBillable = estimatedCostUsd !== null && provider !== "ollama";

  return (
    <div className="rounded-xl border border-border bg-card p-4">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Cpu className="h-3.5 w-3.5" />
          {t("summary.usage.title")}
          {model && <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">{model}</span>}
        </p>
        {isBillable ? (
          <p className="flex items-center gap-1.5 text-sm font-semibold text-foreground">
            <Coins className="h-4 w-4 text-amber-500" />
            {formatUsd(estimatedCostUsd)}
            <span className="text-[10px] font-normal text-muted-foreground">{t("summary.usage.estimated")}</span>
          </p>
        ) : (
          <p className="text-xs text-muted-foreground">{t("summary.usage.free")}</p>
        )}
      </div>
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs sm:grid-cols-4">
        <Stat label={t("summary.usage.prompt")} value={total.promptTokens.toLocaleString()} hint={t("summary.usage.cached", { pct: cachedPct })} />
        <Stat label={t("summary.usage.completion")} value={total.completionTokens.toLocaleString()} hint={t("summary.usage.reasoning", { tokens: total.reasoningTokens.toLocaleString() })} />
        <Stat label={t("summary.usage.total")} value={tokens.toLocaleString()} />
        <Stat label={t("summary.usage.calls")} value={String(calls)} />
      </dl>
    </div>
  );
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-mono font-medium text-foreground">{value}</dd>
      {hint && <dd className="text-[10px] text-muted-foreground">{hint}</dd>}
    </div>
  );
}
