import { useTranslation } from "react-i18next";
import { Gavel, Handshake, Vote } from "lucide-react";
import type { ModeOutcome } from "@/lib/types";
import { cn } from "@/lib/utils";

const SIDE_TONE: Record<string, string> = {
  prosecution: "text-orange-700 dark:text-orange-300",
  defense: "text-sky-700 dark:text-sky-300",
  for: "text-emerald-700 dark:text-emerald-300",
  against: "text-orange-700 dark:text-orange-300",
};

/**
 * The mode's result, as a headline card: the jury's verdict, the parties'
 * decisions on the deal, or the audience's votes before / after the motion.
 */
export function OutcomePanel({ outcome }: { outcome: ModeOutcome | null }) {
  const { t } = useTranslation();
  if (!outcome) return null;
  const side = (s: string | null) => (s ? <span className={cn("font-semibold", SIDE_TONE[s])}>{t(`outcome.side_${s}`, s)}</span> : <span>{t("outcome.noVote")}</span>);

  if (outcome.kind === "verdict") {
    return (
      <section className="rounded-xl border border-border bg-card p-4" aria-label={t("outcome.verdictTitle")}>
        <h2 className="flex items-center gap-2 text-base font-semibold text-foreground">
          <Gavel className="h-4 w-4 text-primary" aria-hidden="true" />
          {t("outcome.verdictTitle")}
        </h2>
        <p className="mt-1 text-sm text-foreground">
          {outcome.winner ? <>{t("outcome.verdictFor")} {side(outcome.winner)}</> : t("outcome.hungJury")}
          {outcome.byArbitre && <span className="ml-2 text-xs text-muted-foreground">({t("outcome.byArbitre")})</span>}
        </p>
        {outcome.votes.length > 0 && (
          <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
            {outcome.votes.map((v) => (
              <li key={v.voterId}>
                <span className="font-medium text-foreground">{v.voterName}</span> : {side(v.choice)}
                {v.reason && <> — {v.reason}</>}
              </li>
            ))}
          </ul>
        )}
      </section>
    );
  }

  if (outcome.kind === "agreement") {
    return (
      <section className={cn("rounded-xl border p-4", outcome.reached ? "border-emerald-500/40 bg-emerald-500/5" : "border-orange-500/40 bg-orange-500/5")} aria-label={t("outcome.agreementTitle")}>
        <h2 className="flex items-center gap-2 text-base font-semibold text-foreground">
          <Handshake className="h-4 w-4 text-primary" aria-hidden="true" />
          {t("outcome.agreementTitle")}
        </h2>
        <p className="mt-1 text-sm text-foreground">{outcome.reached ? t("outcome.agreementReached") : t("outcome.agreementFailed")}</p>
        <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
          {outcome.parties.map((p) => (
            <li key={p.partyId}>
              <span className="font-medium text-foreground">{p.partyName}</span> : {p.accepts ? t("outcome.signs") : t("outcome.refuses")}
              {p.reason && <> — {p.reason}</>}
            </li>
          ))}
        </ul>
      </section>
    );
  }

  return (
    <section className="rounded-xl border border-border bg-card p-4" aria-label={t("outcome.swingTitle")}>
      <h2 className="flex items-center gap-2 text-base font-semibold text-foreground">
        <Vote className="h-4 w-4 text-primary" aria-hidden="true" />
        {t("outcome.swingTitle")}
      </h2>
      <p className="mt-1 text-sm text-foreground">
        {t("outcome.before")} {side(outcome.before)} · {t("outcome.after")} {side(outcome.after)}
      </p>
      <p className="mt-1 text-sm text-foreground">
        {outcome.winner ? <>{t("outcome.swingWinner")} {side(outcome.winner)}</> : t("outcome.noSwing")}
      </p>
    </section>
  );
}
