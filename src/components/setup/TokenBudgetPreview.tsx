import { useTranslation } from "react-i18next";
import { AlertTriangle, CheckCircle, Database, FileText, Info, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TokenBudgetPreview as TokenBudgetPreviewType } from "@/lib/types";

interface Props {
  preview: TokenBudgetPreviewType | null;
  documentInjectionMode: "rag" | "fullInjection";
  hasDocuments: boolean;
}

const SECTION_COLORS: Record<string, string> = {
  currentTurnMessages: "bg-blue-500",
  immediateMemory: "bg-cyan-500",
  contextualSummary: "bg-teal-500",
  cognitiveDirectives: "bg-emerald-500",
  arbitreDirectives: "bg-green-500",
  fullDocument: "bg-purple-500",
  ragContext: "bg-violet-500",
  webWikiSearch: "bg-orange-500",
  positionalMap: "bg-amber-500",
};

const APPROX_CHARS_PER_PAGE = 2_000;

export function TokenBudgetPreviewPanel({ preview, documentInjectionMode, hasDocuments }: Props) {
  const { t } = useTranslation();

  if (!preview) return null;

  const { totalTokens, reservedTokens, availableTokens, sections, warnings, fullDocumentMode, charsPerToken, qualityLevel, documentAvailableTokens } = preview;
  const usedTokens = reservedTokens + Math.round(
    sections.reduce((sum, s) => sum + s.allocatedChars, 0) / charsPerToken,
  );
  const usedPct = totalTokens > 0 ? Math.min(100, (usedTokens / totalTokens) * 100) : 0;

  // Document capacity: actual remaining tokens after all non-document allocations
  const maxDocPages = Math.floor(documentAvailableTokens * charsPerToken / APPROX_CHARS_PER_PAGE);

  return (
    <div className="space-y-3 rounded-lg border border-border bg-card p-4">
      <div className="flex items-center justify-between">
        <h4 className="flex items-center gap-1.5 text-sm font-medium text-foreground">
          <FileText className="h-4 w-4 text-primary" />
          {t("setup.budgetPreview")}
        </h4>
        {qualityLevel === "optimal" && (
          <span className="flex items-center gap-1 rounded-full border border-green-500/30 bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-600">
            <CheckCircle className="h-3 w-3" />
            {t("setup.budgetQualityOptimal")}
          </span>
        )}
        {qualityLevel === "degraded" && (
          <span className="flex items-center gap-1 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-xs font-medium text-amber-600">
            <AlertTriangle className="h-3 w-3" />
            {t("setup.budgetQualityDegraded")}
          </span>
        )}
        {qualityLevel === "insufficient" && (
          <span className="flex items-center gap-1 rounded-full border border-destructive/30 bg-destructive/10 px-2 py-0.5 text-xs font-medium text-destructive">
            <XCircle className="h-3 w-3" />
            {t("setup.budgetQualityInsufficient")}
          </span>
        )}
      </div>

      {/* Pedagogical intro */}
      <p className="text-xs text-muted-foreground leading-relaxed">
        {t("setup.budgetIntro")}
      </p>

      {/* Total budget bar */}
      <div className="space-y-1">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>{t("setup.budgetReserved")}: {reservedTokens.toLocaleString()} tokens</span>
          <span>{t("setup.budgetAvailable")}: {availableTokens.toLocaleString()} tokens</span>
        </div>
        <div className="h-3 w-full overflow-hidden rounded-full bg-muted">
          <div
            className={cn(
              "h-full rounded-full transition-all",
              usedPct > 95 ? "bg-destructive" : usedPct > 80 ? "bg-amber-500" : "bg-primary",
            )}
            style={{ width: `${usedPct}%` }}
          />
        </div>
        <div className="text-right text-xs text-muted-foreground">
          {usedTokens.toLocaleString()} / {totalTokens.toLocaleString()} tokens ({Math.round(usedPct)}%)
        </div>
      </div>

      {/* Document capacity indicator */}
      <div className="flex items-start gap-1.5 rounded-md border border-border/50 bg-muted/30 px-2.5 py-1.5 text-xs text-muted-foreground">
        <Info className="mt-0.5 h-3 w-3 shrink-0 text-primary" />
        <span>{t("setup.budgetDocCapacity", { tokens: documentAvailableTokens.toLocaleString(), pages: maxDocPages })}</span>
      </div>

      {/* Full doc / RAG badge */}
      {hasDocuments && (
        documentInjectionMode === "fullInjection" && fullDocumentMode ? (
          <div className="flex items-center gap-1.5 rounded-md border border-green-500/30 bg-green-500/5 px-2.5 py-1.5 text-xs font-medium text-green-600">
            <FileText className="h-3.5 w-3.5" />
            {t("setup.budgetFullDocMode")}
          </div>
        ) : (
          <div className="flex items-center gap-1.5 rounded-md border border-orange-500/30 bg-orange-500/5 px-2.5 py-1.5 text-xs font-medium text-orange-600">
            <Database className="h-3.5 w-3.5" />
            {t("setup.budgetRagMode")}
          </div>
        )
      )}

      {/* FullInjection requested but budget insufficient warning */}
      {hasDocuments && documentInjectionMode === "fullInjection" && !fullDocumentMode && (
        <div className="flex items-start gap-1.5 rounded-md border border-amber-500/30 bg-amber-500/5 px-2.5 py-1.5 text-xs text-amber-600">
          <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0" />
          <span>{t("setup.injectionModeFullFallback")}</span>
        </div>
      )}

      {/* Per-section bars */}
      <div className="space-y-1.5">
        {sections
          .filter((s) => s.ceilingChars > 0)
          .map((s) => {
            const allocTokens = Math.round(s.allocatedChars / charsPerToken);
            const ceilTokens = Math.round(s.ceilingChars / charsPerToken);
            const pct = ceilTokens > 0 ? Math.round((allocTokens / ceilTokens) * 100) : 0;
            const atFloor = s.allocatedChars <= s.floorChars && s.allocatedChars < s.ceilingChars;
            const colorClass = SECTION_COLORS[s.section] ?? "bg-gray-500";

            return (
              <div key={s.section} className="space-y-0.5">
                <div className="flex items-center justify-between text-xs">
                  <span
                    className={cn("text-muted-foreground", atFloor && "text-amber-500")}
                    title={t(`setup.budgetTooltip_${s.section}`)}
                  >
                    {t(`setup.budgetSection_${s.section}`)}
                  </span>
                  <span className="font-mono text-muted-foreground/70">
                    {allocTokens.toLocaleString()} / {ceilTokens.toLocaleString()} tokens
                  </span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className={cn("h-full rounded-full transition-all", colorClass)}
                    style={{ width: `${Math.min(100, pct)}%` }}
                  />
                </div>
              </div>
            );
          })}
      </div>

      {/* Warnings */}
      {warnings.length > 0 && (
        <div className="space-y-1">
          {warnings.map((w, i) => (
            <div
              key={i}
              className="flex items-start gap-1.5 rounded-md border border-amber-500/30 bg-amber-500/5 px-2.5 py-1.5 text-xs text-amber-600"
            >
              <AlertTriangle className="mt-0.5 h-3 w-3 shrink-0" />
              <span>{w}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
