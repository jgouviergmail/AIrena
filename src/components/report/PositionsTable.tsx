import { useTranslation } from "react-i18next";
import { ArrowRight } from "lucide-react";
import type { PositionRecord } from "@/lib/report";
import { cn } from "@/lib/utils";

/** True when a position visibly moved (mirrors the engine's `has_evolved`). */
export function positionHasEvolved(p: PositionRecord): boolean {
  return Boolean(p.shift?.trim()) || (p.initialStance !== null && p.initialStance !== p.stance);
}

/**
 * Trajectory of every participant's position: where they started, where they
 * ended, how they moved and what would make them move. One card per
 * participant so it reads on a phone; the moved ones are highlighted.
 */
export function PositionsTable({ positions }: { positions: PositionRecord[] }) {
  const { t } = useTranslation();
  if (positions.length === 0) return null;
  return (
    <div className="space-y-2">
      <h2 className="text-lg font-semibold text-foreground">{t("positions.title")}</h2>
      <ul className="grid gap-2 sm:grid-cols-2">
        {positions.map((p) => {
          const evolved = positionHasEvolved(p);
          return (
            <li
              key={p.participantName}
              className={cn("rounded-lg border p-3 text-sm", evolved ? "border-primary/40 bg-primary/5" : "border-border bg-card")}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium text-foreground">{p.participantName}</span>
                <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                  {evolved ? t("positions.shift") : t("positions.unchanged")}
                </span>
              </div>
              {evolved && p.initialStance ? (
                <div className="mt-1 flex flex-wrap items-center gap-1 text-muted-foreground">
                  <span className="line-through decoration-muted-foreground/60">{p.initialStance}</span>
                  <ArrowRight className="h-3 w-3 shrink-0" aria-hidden="true" />
                  <span className="text-foreground">{p.stance}</span>
                </div>
              ) : (
                <p className="mt-1 text-foreground">{p.stance}</p>
              )}
              {p.shift && (
                <p className="mt-1 text-xs text-muted-foreground">
                  <span className="font-medium">{t("positions.shift")}:</span> {p.shift}
                </p>
              )}
              {p.wouldChangeIf && (
                <p className="mt-1 text-xs text-muted-foreground">
                  <span className="font-medium">{t("positions.wouldChangeIf")}:</span> {p.wouldChangeIf}
                </p>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
