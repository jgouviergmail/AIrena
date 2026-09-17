import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { TimelineEntry, TimelineEntryKind } from "@/lib/report";
import { cn } from "@/lib/utils";

const MARK_CLASS: Partial<Record<TimelineEntryKind, string>> = {
  act: "bg-fuchsia-500",
  sceneEvent: "bg-amber-500",
  ban: "bg-destructive",
  banLifted: "bg-emerald-500",
  coalition: "bg-green-500",
  relationshipShift: "bg-sky-500",
  userTurn: "bg-primary",
};

/** Scroll the feed to a turn divider (`#turn-N`). */
export function scrollToTurn(turn: number) {
  document.getElementById(`turn-${turn}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

/**
 * Foot of the feed: one segment per turn with the notable moments as marks;
 * clicking jumps to the turn. Data comes from the store's timeline (persisted
 * in the report) — the same bar works on the replay.
 */
export function TimelineBar({ timeline, turns, currentTurn }: { timeline: TimelineEntry[]; turns: number; currentTurn: number }) {
  const { t } = useTranslation();
  const byTurn = useMemo(() => {
    const map = new Map<number, TimelineEntry[]>();
    for (const e of timeline) {
      map.set(e.turn, [...(map.get(e.turn) ?? []), e]);
    }
    return map;
  }, [timeline]);
  if (turns === 0) return null;

  return (
    <div className="flex shrink-0 items-center gap-1 border-t border-border px-3 py-1.5" role="navigation" aria-label={t("stage.timeline")}>
      {Array.from({ length: turns }, (_, i) => i + 1).map((turn) => {
        const marks = byTurn.get(turn) ?? [];
        const title = marks.length ? marks.map((m) => `${t(`stage.mark_${m.kind}`)}${m.label ? ` · ${m.label}` : ""}`).join("\n") : t("arena.turn", { number: turn });
        return (
          <button
            key={turn}
            type="button"
            onClick={() => scrollToTurn(turn)}
            title={title}
            aria-label={t("arena.turn", { number: turn })}
            className={cn(
              "relative h-3 min-w-4 flex-1 rounded-sm transition-colors hover:bg-accent",
              turn === currentTurn ? "bg-primary/30" : "bg-muted",
            )}
          >
            <span className="absolute inset-x-0 -top-0.5 flex justify-center gap-0.5">
              {marks.slice(0, 4).map((m, i) => (
                <span key={i} className={cn("h-1.5 w-1.5 rounded-full", MARK_CLASS[m.kind] ?? "bg-muted-foreground")} />
              ))}
            </span>
          </button>
        );
      })}
    </div>
  );
}
