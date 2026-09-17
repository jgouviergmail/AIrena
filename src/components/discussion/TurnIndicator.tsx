import { useTranslation } from "react-i18next";
import { Clapperboard, Search, Thermometer } from "lucide-react";
import type { RoomMood } from "@/lib/types";

/** Pill colour per room mood (v1.17). */
const MOOD_CLASS: Record<RoomMood, string> = {
  tense: "bg-red-500/10 text-red-500",
  flat: "bg-slate-500/10 text-slate-500",
  lively: "bg-emerald-500/10 text-emerald-500",
  serene: "bg-sky-500/10 text-sky-500",
};

export function TurnIndicator({
  turn,
  status,
  determiningOrder,
  webSearchCount,
  roomMood,
  actTitle,
}: {
  turn: number;
  status: string;
  determiningOrder: boolean;
  webSearchCount?: number;
  /** Temperature of the room, when at least two gladiateurs are active */
  roomMood?: RoomMood | null;
  /** Title of the act in progress (localised by the engine) */
  actTitle?: string | null;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2">
      {actTitle && (
        <span className="inline-flex items-center gap-1 rounded-full bg-fuchsia-500/10 px-2.5 py-0.5 text-xs font-medium text-fuchsia-500" title={t("arena.currentAct")}>
          <Clapperboard className="h-3 w-3" />
          {actTitle}
        </span>
      )}
      {roomMood && (
        <span className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium ${MOOD_CLASS[roomMood]}`} title={t("arena.roomMood")}>
          <Thermometer className="h-3 w-3" />
          {t(`arena.mood_${roomMood}`)}
        </span>
      )}
      {turn > 0 && (
        <span className="rounded-full bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-primary">
          {t("arena.turn", { number: turn })}
        </span>
      )}
      {(webSearchCount ?? 0) > 0 && (
        <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2.5 py-0.5 text-xs font-medium text-blue-500">
          <Search className="h-3 w-3" />
          {webSearchCount}
        </span>
      )}
      {status === "paused" && (
        <span className="rounded-full bg-amber-500/10 px-2.5 py-0.5 text-xs font-medium text-amber-500">
          {t("arena.paused")}
        </span>
      )}
      {status === "synthesizing" && (
        <span className="rounded-full bg-blue-500/10 px-2.5 py-0.5 text-xs font-medium text-blue-500">
          {t("arena.synthesizing")}
        </span>
      )}
      {determiningOrder && (
        <span className="flex items-center gap-1.5 rounded-full bg-violet-500/10 px-2.5 py-0.5 text-xs font-medium text-violet-500">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-violet-500 animate-pulse" />
          {t("arena.determiningOrder")}
        </span>
      )}
    </div>
  );
}
