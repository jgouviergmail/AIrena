import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useArenaStore } from "@/stores/useArenaStore";
import { BANNER_MS } from "@/lib/stage";
import { cn } from "@/lib/utils";

const BANNER_CLASS = {
  turn: "bg-primary/90 text-primary-foreground",
  act: "bg-fuchsia-600/90 text-white",
  sceneEvent: "bg-amber-500/90 text-black",
  ban: "bg-destructive/90 text-white",
} as const;

/**
 * Full-width announcement over the feed (turn, act, scene event, ban), gone
 * after `BANNER_MS`. Announced to assistive tech through `aria-live`; the
 * slide animation only plays when motion is allowed.
 */
export function SceneBanner() {
  const { t } = useTranslation();
  const banner = useArenaStore((s) => s.banner);
  const dismiss = useArenaStore((s) => s.dismissBanner);

  useEffect(() => {
    if (!banner) return;
    const timer = window.setTimeout(() => dismiss(banner.id), BANNER_MS);
    return () => window.clearTimeout(timer);
  }, [banner, dismiss]);

  const text = banner
    ? banner.kind === "turn"
      ? t("arena.turn", { number: banner.text })
      : banner.kind === "sceneEvent"
        ? t(`stage.event_${banner.text}`, banner.text)
        : banner.kind === "ban"
          ? t("stage.bannerBan", { name: banner.text })
          : banner.text
    : "";

  return (
    <div aria-live="polite" className="pointer-events-none absolute inset-x-0 top-2 z-20 flex justify-center px-4">
      {banner && (
        <div
          key={banner.id}
          className={cn(
            "max-w-full line-clamp-2 rounded-2xl px-5 py-1.5 text-center font-display text-sm font-semibold shadow-lg motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-2",
            BANNER_CLASS[banner.kind],
          )}
        >
          {text}
        </div>
      )}
    </div>
  );
}
