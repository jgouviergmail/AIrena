import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Ban, Check, Loader2, MinusCircle } from "lucide-react";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { cn } from "@/lib/utils";

export type QueueState = "done" | "active" | "waiting" | "banned" | "passed";

export interface QueueEntry {
  id: string;
  state: QueueState;
}

/**
 * Pure derivation of the speaking queue for the current turn: the announced
 * order, each speaker's state, plus participants who passed (UserDriven).
 */
export function deriveQueue(input: {
  speakerOrder: string[];
  activeSpeakerId: string | null;
  spokenThisTurn: Set<string>;
  bannedIds: Set<string>;
  passedIds: string[];
}): QueueEntry[] {
  const entries: QueueEntry[] = input.speakerOrder.map((id) => ({
    id,
    state: input.bannedIds.has(id)
      ? "banned"
      : id === input.activeSpeakerId && !input.spokenThisTurn.has(id)
        ? "active"
        : input.spokenThisTurn.has(id)
          ? "done"
          : "waiting",
  }));
  for (const id of input.passedIds) {
    if (!entries.some((e) => e.id === id)) entries.push({ id, state: "passed" });
  }
  return entries;
}

const STATE_CLASS: Record<QueueState, string> = {
  done: "border-border bg-muted/40 text-muted-foreground",
  active: "border-primary bg-primary/10 text-primary",
  waiting: "border-border text-foreground",
  banned: "border-destructive/40 bg-destructive/5 text-destructive line-through",
  passed: "border-dashed border-border text-muted-foreground",
};

/** Chips for the turn's speaking order — who spoke, who is speaking, who is next. */
export function SpeakerQueue() {
  const { t } = useTranslation();
  const speakerOrder = useArenaStore((s) => s.speakerOrder);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const messages = useArenaStore((s) => s.messages);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const bans = useArenaStore((s) => s.bans);
  const passedSpeakerIds = useArenaStore((s) => s.passedSpeakerIds);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);

  const entries = useMemo(() => {
    const spoken = new Set(
      messages.filter((m) => m.turnNumber === currentTurn && m.role === "GladIAteur").map((m) => m.speakerId),
    );
    const banned = new Set([...bans.entries()].filter(([, b]) => b.remaining > 0).map(([id]) => id));
    return deriveQueue({ speakerOrder, activeSpeakerId, spokenThisTurn: spoken, bannedIds: banned, passedIds: passedSpeakerIds });
  }, [speakerOrder, activeSpeakerId, messages, currentTurn, bans, passedSpeakerIds]);

  if (entries.length === 0) return null;

  const describe = (id: string) => {
    if (id === arbitre.id) return { name: arbitre.name, emoji: ROLE_EMOJIS.IArbitre };
    const g = gladiateurs.find((x) => x.id === id);
    return g ? { name: g.name, emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt) } : { name: id, emoji: "" };
  };

  return (
    <div className="flex min-w-0 items-center gap-1 overflow-x-auto" aria-label={t("arena.queue.label")}>
      {entries.map((e) => {
        const { name, emoji } = describe(e.id);
        return (
          <span
            key={e.id}
            title={`${name} — ${t(`arena.queue.${e.state}`)}`}
            className={cn(
              "inline-flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] transition-colors",
              STATE_CLASS[e.state],
            )}
          >
            {e.state === "active" && <Loader2 className="h-3 w-3 animate-spin" />}
            {e.state === "done" && <Check className="h-3 w-3" />}
            {e.state === "banned" && <Ban className="h-3 w-3" />}
            {e.state === "passed" && <MinusCircle className="h-3 w-3" />}
            <span>{emoji}</span>
            <span className="hidden max-w-28 truncate sm:inline">{name}</span>
          </span>
        );
      })}
    </div>
  );
}
