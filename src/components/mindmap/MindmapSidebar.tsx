import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Loader2, LocateFixed, Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import { MarkmapViewer, type MarkmapViewerHandle } from "@/components/mindmap/MarkmapViewer";
import { useArenaStore } from "@/stores/useArenaStore";
import type { ArgumentMap, ArgumentNode } from "@/lib/types";

type ArgMapView = "thesis" | "speaker";

/** Number of nodes (theses + arguments, recursively) each speaker contributed. */
export function countNodesBySpeaker(map: ArgumentMap | null): { name: string; count: number }[] {
  if (!map) return [];
  const counts = new Map<string, number>();
  const bump = (name: string) => counts.set(name, (counts.get(name) ?? 0) + 1);
  const walk = (arg: ArgumentNode) => {
    bump(arg.speakerName);
    arg.children.forEach(walk);
  };
  for (const thesis of map.theses) {
    bump(thesis.speakerName);
    thesis.arguments.forEach(walk);
  }
  return [...counts.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count);
}

/** Argument map panel content: view toggle, live map, per-speaker chips, legend. */
export function ArgumentMapPanel() {
  const { t } = useTranslation();
  const [view, setView] = useState<ArgMapView>("thesis");
  const viewerRef = useRef<MarkmapViewerHandle>(null);
  const markdown = useArenaStore((s) => s.argumentMapMarkdown);
  const markdownBySpeaker = useArenaStore((s) => s.argumentMapMarkdownBySpeaker);
  const thesesCount = useArenaStore((s) => s.argumentMapThesesCount);
  const argumentsCount = useArenaStore((s) => s.argumentMapArgumentsCount);
  const argumentMap = useArenaStore((s) => s.argumentMap);
  const newNodeIds = useArenaStore((s) => s.argumentMapNewNodeIds);
  const droppedCount = useArenaStore((s) => s.argumentMapDroppedCount);
  const analysing = useArenaStore((s) => s.activityStatus?.type === "argumentMap");
  const bySpeaker = useMemo(() => countNodesBySpeaker(argumentMap), [argumentMap]);

  const activeMarkdown = view === "thesis" ? markdown : markdownBySpeaker;

  return (
    <>
      {/* Counters */}
      <div className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-border px-3 py-1.5 text-[10px] text-muted-foreground">
        <span>{t("mindmap.thesesCount", { count: thesesCount })}</span>
        <span>{t("mindmap.argumentsCount", { count: argumentsCount })}</span>
        {newNodeIds.length > 0 && (
          <span className="inline-flex items-center gap-0.5 text-primary">
            <Sparkles className="h-3 w-3" />
            {t("mindmap.newThisTurn", { count: newNodeIds.length })}
          </span>
        )}
        {droppedCount > 0 && (
          <span className="inline-flex items-center gap-0.5 text-amber-500" title={t("mindmap.droppedHint")}>
            <AlertTriangle className="h-3 w-3" />
            {t("mindmap.dropped", { count: droppedCount })}
          </span>
        )}
        {analysing && (
          <span className="inline-flex items-center gap-1 text-primary">
            <Loader2 className="h-3 w-3 animate-spin" />
            {t("mindmap.analysing")}
          </span>
        )}
        <button
          onClick={() => viewerRef.current?.fit()}
          className="ml-auto inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("mindmap.recenter")}
        >
          <LocateFixed className="h-3 w-3" />
          {t("mindmap.recenter")}
        </button>
      </div>

      {/* Per-speaker chips */}
      {bySpeaker.length > 0 && (
        <div className="flex shrink-0 flex-wrap gap-1 border-b border-border px-3 py-1.5">
          {bySpeaker.map((s) => (
            <span key={s.name} className="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary">
              {s.name} · {s.count}
            </span>
          ))}
        </div>
      )}

      {/* View toggle */}
      {markdown && (
        <div className="flex shrink-0 gap-1 border-b border-border px-3 py-1.5">
          <button
            onClick={() => setView("thesis")}
            className={cn(
              "flex-1 rounded-md px-2 py-1 text-[11px] font-medium transition-colors",
              view === "thesis"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {t("mindmap.viewByThesis")}
          </button>
          <button
            onClick={() => setView("speaker")}
            className={cn(
              "flex-1 rounded-md px-2 py-1 text-[11px] font-medium transition-colors",
              view === "speaker"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {t("mindmap.viewBySpeaker")}
          </button>
        </div>
      )}

      {/* Mind map content */}
      <div className="flex-1 overflow-hidden">
        {activeMarkdown ? (
          <MarkmapViewer ref={viewerRef} markdown={activeMarkdown} />
        ) : (
          <div className="flex h-full items-center justify-center px-4">
            <p className="text-center text-sm text-muted-foreground italic">
              {t("mindmap.empty")}
            </p>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-t border-border px-3 py-1">
        <span className="text-[10px] text-muted-foreground">
          ✅ {t("mindmap.legendSupport")}
        </span>
        <span className="text-[10px] text-muted-foreground">
          ❌ {t("mindmap.legendCounter")}
        </span>
        <span className="text-[10px] text-muted-foreground">
          📊 {t("mindmap.legendEvidence")}
        </span>
        <span className="text-[10px] text-muted-foreground">
          ✨ {t("mindmap.legendNew")}
        </span>
      </div>
    </>
  );
}
