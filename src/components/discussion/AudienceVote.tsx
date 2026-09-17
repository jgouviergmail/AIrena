import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ThumbsDown, ThumbsUp, Vote } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { VOTE_CHOICES, type VoteChoice } from "@/lib/modes";
import { cn } from "@/lib/utils";
import { toast } from "@/stores/useToastStore";
import { useArenaStore } from "@/stores/useArenaStore";
import type { VotePhase } from "@/lib/types";

const CHOICE_ICONS: Record<VoteChoice, typeof ThumbsUp> = { for: ThumbsUp, against: ThumbsDown };

/**
 * The audience's vote on the motion of an Oxford debate: asked once before the
 * first turn and once after the last; the window closes on the user turn timeout.
 */
export function AudienceVote({ phase, timeoutSecs }: { phase: VotePhase; timeoutSecs: number }) {
  const { t } = useTranslation();
  const [remaining, setRemaining] = useState(timeoutSecs);
  const [sending, setSending] = useState(false);

  useEffect(() => {
    setRemaining(timeoutSecs);
    const interval = setInterval(() => setRemaining((prev) => (prev <= 1 ? 0 : prev - 1)), 1000);
    return () => clearInterval(interval);
  }, [phase, timeoutSecs]);

  const send = async (action: () => Promise<void>) => {
    if (sending) return;
    setSending(true);
    try {
      await action();
    } catch (e: unknown) {
      toast.error(t("vote.failed"), extractErrorMessage(e));
      setSending(false);
    }
  };
  const vote = (choice: VoteChoice) => send(() => api.audienceVote(choice));
  // Declining closes the window at once (the engine treats it like a skipped user
  // turn and emits nothing: the window is closed here)
  const decline = () => send(async () => {
    await api.skipUserTurn();
    useArenaStore.setState({ voteRequest: null });
  });

  return (
    <div className="border-t border-primary/30 bg-primary/5 p-3 sm:p-4" role="group" aria-label={t("vote.title")}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="flex items-center gap-2 text-sm font-medium text-foreground">
          <Vote className="h-4 w-4 text-primary" aria-hidden="true" />
          {t(`vote.ask_${phase}`)}
        </p>
        <span className="text-xs text-muted-foreground">{t("vote.remaining", { seconds: remaining })}</span>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        {VOTE_CHOICES.map((choice) => {
          const Icon = CHOICE_ICONS[choice];
          return (
            <button
              key={choice}
              type="button"
              onClick={() => vote(choice)}
              disabled={sending}
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm font-medium transition-colors disabled:opacity-50",
                choice === "for" ? "border-emerald-500/40 text-emerald-700 hover:bg-emerald-500/10 dark:text-emerald-300" : "border-orange-500/40 text-orange-700 hover:bg-orange-500/10 dark:text-orange-300",
              )}
            >
              <Icon className="h-4 w-4" aria-hidden="true" />
              {t(`vote.${choice}`)}
            </button>
          );
        })}
        <button
          type="button"
          onClick={decline}
          disabled={sending}
          className="rounded-md px-3 py-1.5 text-sm text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-50"
        >
          {t("vote.decline")}
        </button>
      </div>
    </div>
  );
}
