import { useTranslation } from "react-i18next";
import { Coins, Cpu, Zap } from "lucide-react";
import { useArenaStore } from "@/stores/useArenaStore";
import { formatTokens, formatUsd } from "@/lib/cost-estimate";
import { cn } from "@/lib/utils";

/** Live token / cost indicator fed by `llmUsageUpdated` events. */
export function UsagePill() {
  const { t } = useTranslation();
  const usage = useArenaStore((s) => s.llmUsage);
  if (!usage) return null;

  const total = usage.total.promptTokens + usage.total.completionTokens;
  const cached = usage.total.promptTokens > 0 ? Math.round((usage.total.cachedTokens / usage.total.promptTokens) * 100) : 0;
  const overBudget = usage.budgetUsd > 0 && usage.estimatedCostUsd !== null && usage.periodSpentUsd + usage.estimatedCostUsd >= usage.budgetUsd;
  const title = [
    t("arena.usage.prompt", { tokens: usage.total.promptTokens.toLocaleString(), cached }),
    t("arena.usage.completion", { tokens: usage.total.completionTokens.toLocaleString(), reasoning: usage.total.reasoningTokens.toLocaleString() }),
    t("arena.usage.calls", { calls: usage.calls }),
    usage.estimatedCostUsd !== null
      ? t("arena.usage.cost", { cost: formatUsd(usage.estimatedCostUsd), period: formatUsd(usage.periodSpentUsd + usage.estimatedCostUsd), budget: usage.budgetUsd > 0 ? formatUsd(usage.budgetUsd) : "∞" })
      : t("arena.usage.free"),
    usage.provider === "deepseek" ? (usage.peak ? t("settings.deepseekPeakNow") : t("settings.deepseekOffpeakNow")) : "",
  ].filter(Boolean).join("\n");

  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium",
        overBudget ? "bg-destructive/10 text-destructive" : "bg-muted text-muted-foreground",
      )}
    >
      <Cpu className="h-3 w-3" />
      <span className="font-mono">{formatTokens(total)}</span>
      {usage.estimatedCostUsd !== null && (
        <>
          <Coins className="h-3 w-3 text-amber-500" />
          <span className="font-mono">{formatUsd(usage.estimatedCostUsd)}</span>
          {usage.peak && <Zap className="h-3 w-3 text-amber-500" aria-label={t("settings.deepseekPeakNow")} />}
        </>
      )}
    </span>
  );
}
