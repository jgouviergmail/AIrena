import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { computeScores } from "@/lib/score";
import { spokenParticipants, USER_SPEAKER_ID } from "@/lib/stage";
import { cn } from "@/lib/utils";

const MEDALS = ["🥇", "🥈", "🥉"];

/** Points per turn and the ranking of the gladiateurs (arena tab "Score"). */
export function Scoreboard() {
  const { t } = useTranslation();
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const messages = useArenaStore((s) => s.messages);
  const argumentMap = useArenaStore((s) => s.argumentMap);
  const bans = useArenaStore((s) => s.bans);
  const concessions = useArenaStore((s) => s.concessions);
  const currentTurn = useArenaStore((s) => s.currentTurn);

  const rows = useMemo(
    () => {
      const cast = gladiateurs.map((g) => ({ id: g.id, name: g.name }));
      // The audience member is ranked too once they have spoken (v1.20.2)
      const audience = spokenParticipants(messages).filter((p) => p.id === USER_SPEAKER_ID);
      return computeScores({ messages, argumentMap, bans, concessions, participants: [...cast, ...audience] });
    },
    [messages, argumentMap, bans, concessions, gladiateurs],
  );
  const emoji = useMemo(() => new Map<string, string>([...gladiateurs.map((g) => [g.id, g.emoji ?? getProfileEmoji(g.name, g.systemPrompt)] as [string, string]), [USER_SPEAKER_ID, ROLE_EMOJIS.user]]), [gladiateurs]);
  const turns = Array.from({ length: currentTurn }, (_, i) => i + 1);

  if (rows.length === 0) return <p className="p-4 text-center text-sm italic text-muted-foreground">{t("score.empty")}</p>;

  return (
    <div className="flex-1 space-y-3 overflow-y-auto p-3">
      <ol className="space-y-2">
        {rows.map((r) => (
          <li key={r.speakerId} className={cn("rounded-lg border p-2.5", r.rank === 1 ? "border-primary/40 bg-primary/5" : "border-border bg-card")}>
            <div className="flex items-center gap-2">
              <span className="w-6 text-center text-base" aria-label={t("score.rank", { rank: r.rank })}>{MEDALS[r.rank - 1] ?? r.rank}</span>
              <span aria-hidden="true">{emoji.get(r.speakerId)}</span>
              <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">{r.speakerName}</span>
              <span className="font-mono text-sm font-semibold text-foreground">{r.points}</span>
            </div>
            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-muted-foreground">
              <span>{t("score.reactions")}: {r.reactions}</span>
              <span>{t("score.arguments")}: {r.arguments}</span>
              {r.concessions !== 0 && <span>{t("score.concessions")}: {r.concessions}</span>}
              {r.bans !== 0 && <span className="text-destructive">{t("score.bans")}: {r.bans}</span>}
            </div>
          </li>
        ))}
      </ol>
      {turns.length > 1 && (
        <div className="overflow-x-auto">
          <table className="w-full text-[10px]">
            <thead className="text-muted-foreground">
              <tr>
                <th className="py-1 text-left font-medium">{t("score.byTurn")}</th>
                {turns.map((turn) => <th key={turn} className="py-1 text-right font-medium">{turn}</th>)}
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.speakerId} className="border-t border-border/60">
                  <td className="max-w-[90px] truncate py-1 text-foreground">{r.speakerName}</td>
                  {turns.map((turn) => <td key={turn} className="py-1 text-right font-mono tabular-nums text-foreground">{r.byTurn[turn] ?? 0}</td>)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
