import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, ChevronDown, ChevronUp } from "lucide-react";
import type { DiscussionDiagnostics, TurnTimings } from "@/lib/report";

/** Phases shown as columns of the timings table (the others are folded into "end of turn"). */
const TIMING_COLUMNS = ["speakers", "endOfTurn"] as const;

function formatMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${ms} ms`;
}

/** Duration of a named phase within a turn (0 when the phase did not run). */
export function phaseMs(t: TurnTimings, name: string): number {
  return t.phases.filter((p) => p.name === name).reduce((sum, p) => sum + p.ms, 0);
}

/** Mean wall-clock time of a phase across turns (null without data). */
export function averagePhaseMs(timings: TurnTimings[], name: string): number | null {
  const values = timings.map((t) => phaseMs(t, name)).filter((ms) => ms > 0);
  return values.length ? Math.round(values.reduce((a, b) => a + b, 0) / values.length) : null;
}

/**
 * Collapsed by default: what the engine measured (timings per turn) and what
 * went wrong or not (parse failures, refusals, retries, intention compliance).
 */
export function DiagnosticsPanel({ diagnostics, timings }: { diagnostics: DiscussionDiagnostics | null; timings: TurnTimings[] }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  if (!diagnostics && timings.length === 0) return null;
  const failures = diagnostics ? Object.entries(diagnostics.jsonParseFailures) : [];
  const totalFailures = failures.reduce((n, [, v]) => n + v, 0);
  const compliance = diagnostics?.intentionCompliance ?? null;

  return (
    <div className="rounded-xl border border-border bg-card">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className="flex w-full items-center gap-2 px-4 py-3 text-left text-sm font-medium text-foreground"
      >
        <Activity className="h-4 w-4 text-primary" />
        <span className="flex-1">{t("diagnostics.title")}</span>
        {diagnostics && (
          <span className="text-xs text-muted-foreground">
            {t("diagnostics.summaryLine", { failures: totalFailures, refusals: diagnostics.refusals, retries: diagnostics.retries })}
          </span>
        )}
        {open ? <ChevronUp className="h-4 w-4 text-muted-foreground" /> : <ChevronDown className="h-4 w-4 text-muted-foreground" />}
      </button>
      {open && (
        <div className="space-y-4 border-t border-border px-4 py-3 text-sm">
          {diagnostics && (
            <dl className="grid grid-cols-2 gap-x-4 gap-y-2 sm:grid-cols-4">
              <div>
                <dt className="text-xs text-muted-foreground">{t("diagnostics.parseFailures")}</dt>
                <dd className="font-medium text-foreground">{totalFailures}</dd>
                {failures.length > 0 && (
                  <dd className="text-xs text-muted-foreground">
                    {failures.map(([kind, n]) => `${t(`diagnostics.kinds.${kind}`, kind)} ×${n}`).join(", ")}
                  </dd>
                )}
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">{t("diagnostics.refusals")}</dt>
                <dd className="font-medium text-foreground">{diagnostics.refusals}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">{t("diagnostics.retries")}</dt>
                <dd className="font-medium text-foreground">{diagnostics.retries}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">{t("diagnostics.intentionCompliance")}</dt>
                <dd className="font-medium text-foreground">{compliance === null ? "—" : `${Math.round(compliance * 100)} %`}</dd>
              </div>
            </dl>
          )}
          {timings.length > 0 && (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead className="text-muted-foreground">
                  <tr>
                    <th className="py-1 text-left font-medium">{t("diagnostics.turn")}</th>
                    {TIMING_COLUMNS.map((c) => (
                      <th key={c} className="py-1 text-right font-medium">{t(`diagnostics.phases.${c}`)}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {timings.map((row) => (
                    <tr key={row.turn} className="border-t border-border/60">
                      <td className="py-1 text-foreground">{row.turn}</td>
                      {TIMING_COLUMNS.map((c) => (
                        <td key={c} className="py-1 text-right tabular-nums text-foreground">{formatMs(phaseMs(row, c))}</td>
                      ))}
                    </tr>
                  ))}
                  <tr className="border-t border-border text-muted-foreground">
                    <td className="py-1">{t("diagnostics.average")}</td>
                    {TIMING_COLUMNS.map((c) => {
                      const avg = averagePhaseMs(timings, c);
                      return <td key={c} className="py-1 text-right tabular-nums">{avg === null ? "—" : formatMs(avg)}</td>;
                    })}
                  </tr>
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
