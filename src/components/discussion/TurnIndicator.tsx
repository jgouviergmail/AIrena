import { useTranslation } from "react-i18next";

export function TurnIndicator({
  turn,
  status,
}: {
  turn: number;
  status: string;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2">
      {turn > 0 && (
        <span className="rounded-full bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-primary">
          {t("arena.turn", { number: turn })}
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
    </div>
  );
}
