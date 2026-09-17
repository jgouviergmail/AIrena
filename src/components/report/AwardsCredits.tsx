import { useTranslation } from "react-i18next";
import { Trophy } from "lucide-react";
import type { Award } from "@/lib/report";
import type { ParticipantInfo } from "@/lib/types";

const AWARD_EMOJI: Record<string, string> = {
  bestArgument: "🏆",
  biggestShift: "🔄",
  mostContested: "🔥",
  mostProlific: "🧠",
  reconciliation: "🤝",
};

/** Delay between two trophies of the credits (ms). */
const STAGGER_MS = 220;

/** The closing credits: one trophy per award, revealed one after the other. */
export function AwardsCredits({ awards, participants }: { awards: Award[]; participants: ParticipantInfo[] }) {
  const { t } = useTranslation();
  if (awards.length === 0) return null;
  const emoji = new Map(participants.map((p) => [p.id, p.emoji]));
  return (
    <div className="rounded-xl border border-border bg-card p-4">
      <h2 className="mb-3 flex items-center gap-2 font-display text-lg font-semibold text-foreground">
        <Trophy className="h-4 w-4 text-amber-500" />
        {t("awards.title")}
      </h2>
      <ul className="grid gap-2 sm:grid-cols-2">
        {awards.map((a, i) => (
          <li
            key={a.key}
            className="flex items-start gap-3 rounded-lg border border-border bg-background/60 p-3 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2 motion-safe:fill-mode-both"
            style={{ animationDelay: `${i * STAGGER_MS}ms` }}
          >
            <span className="text-2xl" aria-hidden="true">{AWARD_EMOJI[a.key] ?? "🏅"}</span>
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{t(`awards.${a.key}`, a.key)}</p>
              <p className="text-sm font-semibold text-foreground">
                {emoji.get(a.speakerId) && <span className="mr-1" aria-hidden="true">{emoji.get(a.speakerId)}</span>}
                {a.speakerName}
              </p>
              {a.detail && <p className="mt-0.5 break-words text-xs leading-snug text-muted-foreground">{a.detail}</p>}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
