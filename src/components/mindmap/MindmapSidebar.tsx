import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight, Network } from "lucide-react";
import { cn } from "@/lib/utils";
import { MarkmapViewer } from "@/components/mindmap/MarkmapViewer";
import { useArenaStore } from "@/stores/useArenaStore";

type ArgMapView = "thesis" | "speaker";

export function MindmapSidebar({ width = 350 }: { width?: number }) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [view, setView] = useState<ArgMapView>("thesis");
  const markdown = useArenaStore((s) => s.argumentMapMarkdown);
  const markdownBySpeaker = useArenaStore(
    (s) => s.argumentMapMarkdownBySpeaker,
  );
  const thesesCount = useArenaStore((s) => s.argumentMapThesesCount);
  const argumentsCount = useArenaStore((s) => s.argumentMapArgumentsCount);

  const activeMarkdown = view === "thesis" ? markdown : markdownBySpeaker;

  if (isCollapsed) {
    return (
      <div className="flex w-8 flex-col items-center border-l border-border bg-card pt-2">
        <button
          onClick={() => setIsCollapsed(false)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("mindmap.expand")}
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="mt-2 text-xs [writing-mode:vertical-lr] text-muted-foreground">
          {t("mindmap.title")}
        </span>
      </div>
    );
  }

  return (
    <div
      className="flex shrink-0 flex-col border-l border-border bg-card"
      style={{ width: `${width}px` }}
    >
      {/* Header */}
      <div className="flex shrink-0 items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <Network className="h-4 w-4 text-primary" />
          <span className="text-xs font-medium text-foreground">
            {t("mindmap.title")}
          </span>
          {(thesesCount > 0 || argumentsCount > 0) && (
            <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
              {thesesCount}T / {argumentsCount}A
            </span>
          )}
        </div>
        <button
          onClick={() => setIsCollapsed(true)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("mindmap.collapse")}
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>

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
          <MarkmapViewer markdown={activeMarkdown} />
        ) : (
          <div className="flex h-full items-center justify-center px-4">
            <p className="text-center text-sm text-muted-foreground italic">
              {t("mindmap.empty")}
            </p>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="flex shrink-0 items-center gap-4 border-t border-border px-3 py-1">
        <span className="text-[10px] text-muted-foreground">
          ✅ {t("mindmap.legendSupport")}
        </span>
        <span className="text-[10px] text-muted-foreground">
          ❌ {t("mindmap.legendCounter")}
        </span>
        <span className="text-[10px] text-muted-foreground">
          📊 {t("mindmap.legendEvidence")}
        </span>
      </div>
    </div>
  );
}
