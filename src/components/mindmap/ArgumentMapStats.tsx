import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { countNodesBySpeaker } from "./MindmapSidebar";
import type { ArgumentMap } from "@/lib/types";

/** Parse a persisted `argumentMapJson`; empty or malformed → null (older discussions). */
export function parseArgumentMapJson(json: string | null | undefined): ArgumentMap | null {
  if (!json) return null;
  try {
    const parsed = JSON.parse(json) as Partial<ArgumentMap>;
    return Array.isArray(parsed.theses) ? { theses: parsed.theses } : null;
  } catch {
    return null;
  }
}

/** Counts + per-speaker contribution chips derived from the structured map. */
export function ArgumentMapStats({ map }: { map: ArgumentMap | null }) {
  const { t } = useTranslation();
  const bySpeaker = useMemo(() => countNodesBySpeaker(map), [map]);
  if (!map || map.theses.length === 0) return null;
  const argumentsCount = bySpeaker.reduce((sum, s) => sum + s.count, 0) - map.theses.length;
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 px-2 pb-2 text-[11px] text-muted-foreground">
      <span>{t("mindmap.thesesCount", { count: map.theses.length })}</span>
      <span>{t("mindmap.argumentsCount", { count: argumentsCount })}</span>
      {bySpeaker.map((s) => (
        <span key={s.name} className="rounded-full bg-primary/10 px-2 py-0.5 font-medium text-primary">
          {s.name} · {s.count}
        </span>
      ))}
    </div>
  );
}
