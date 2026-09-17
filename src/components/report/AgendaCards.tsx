import { useTranslation } from "react-i18next";
import { CircleCheck, CircleHelp, CircleX, EyeOff } from "lucide-react";
import type { AgendaRecord } from "@/lib/report";
import { cn } from "@/lib/utils";

const VERDICT = {
  achieved: { icon: CircleCheck, className: "border-emerald-500/40 bg-emerald-500/5", badge: "text-emerald-700 dark:text-emerald-300" },
  notAchieved: { icon: CircleX, className: "border-orange-500/40 bg-orange-500/5", badge: "text-orange-700 dark:text-orange-300" },
  unknown: { icon: CircleHelp, className: "border-border bg-card", badge: "text-muted-foreground" },
} as const;

function verdictOf(a: AgendaRecord): keyof typeof VERDICT {
  if (a.achieved === true) return "achieved";
  if (a.achieved === false) return "notAchieved";
  return "unknown";
}

/**
 * The secret agendas unveiled at the end: what each gladiateur was really
 * after, what they would never concede, and the moderator's verdict.
 */
export function AgendaCards({ agendas }: { agendas: AgendaRecord[] }) {
  const { t } = useTranslation();
  if (agendas.length === 0) return null;
  return (
    <div className="space-y-2">
      <h2 className="flex items-center gap-2 text-lg font-semibold text-foreground">
        <EyeOff className="h-4 w-4 text-primary" aria-hidden="true" />
        {t("agendas.title")}
      </h2>
      <p className="text-xs text-muted-foreground">{t("agendas.hint")}</p>
      <ul className="grid gap-2 sm:grid-cols-2">
        {agendas.map((a) => {
          const verdict = verdictOf(a);
          const { icon: Icon, className, badge } = VERDICT[verdict];
          return (
            <li key={a.speakerId} className={cn("rounded-lg border p-3 text-sm", className)}>
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium text-foreground">{a.speakerName}</span>
                <span className={cn("inline-flex items-center gap-1 text-[11px] uppercase tracking-wide", badge)}>
                  <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                  {t(`agendas.${verdict}`)}
                </span>
              </div>
              {a.objective && <p className="mt-1 text-foreground"><span className="font-medium">{t("agendas.objective")}:</span> {a.objective}</p>}
              {a.redLine && <p className="mt-1 text-xs text-muted-foreground"><span className="font-medium">{t("agendas.redLine")}:</span> {a.redLine}</p>}
              {a.victory && <p className="mt-1 text-xs text-muted-foreground"><span className="font-medium">{t("agendas.victory")}:</span> {a.victory}</p>}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
