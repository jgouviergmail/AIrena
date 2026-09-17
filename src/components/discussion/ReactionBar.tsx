import { useTranslation } from "react-i18next";
import { REACTION_EMOJI } from "@/lib/reactions";
import { REACTION_TYPES, type ReactionType } from "@/lib/types";
import { cn } from "@/lib/utils";

/** Audience reaction buttons under a message (arena only, while the discussion runs). */
export function ReactionBar({
  onReact,
  used,
  max,
}: {
  onReact: (type: ReactionType) => void;
  /** Reactions the audience already gave to this message */
  used: number;
  /** Cap per message (engine constant) */
  max: number;
}) {
  const { t } = useTranslation();
  const exhausted = used >= max;
  return (
    <div className="mt-2 flex flex-wrap items-center gap-1" role="group" aria-label={t("arena.react")}>
      {REACTION_TYPES.map((type) => (
        <button
          key={type}
          type="button"
          disabled={exhausted}
          onClick={() => onReact(type)}
          title={exhausted ? t("arena.reactMax", { count: used }) : t(`arena.reactAs_${type}`)}
          aria-label={t(`arena.reactAs_${type}`)}
          className={cn(
            "rounded-full border border-border px-1.5 py-0.5 text-xs leading-none transition-colors",
            "hover:bg-accent motion-safe:hover:scale-110 disabled:cursor-not-allowed disabled:opacity-40",
          )}
        >
          {REACTION_EMOJI[type]}
        </button>
      ))}
      {used > 0 && <span className="ml-1 text-[10px] text-muted-foreground">{used}/{max}</span>}
    </div>
  );
}
