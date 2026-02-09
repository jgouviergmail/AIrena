import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";

export function TurnIndicator({
  turn,
  status,
  determiningOrder,
  webSearchCount,
}: {
  turn: number;
  status: string;
  determiningOrder: boolean;
  webSearchCount?: number;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2">
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
