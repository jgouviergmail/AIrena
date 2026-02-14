import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight, Network } from "lucide-react";
import { MarkmapViewer } from "@/components/mindmap/MarkmapViewer";
import { useArenaStore } from "@/stores/useArenaStore";

export function MindmapSidebar({ width = 350 }: { width?: number }) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const markdown = useArenaStore((s) => s.argumentMapMarkdown);
  const thesesCount = useArenaStore((s) => s.argumentMapThesesCount);
  const argumentsCount = useArenaStore((s) => s.argumentMapArgumentsCount);

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

      {/* Mind map content */}
      <div className="flex-1 overflow-hidden">
        {markdown ? (
          <MarkmapViewer markdown={markdown} />
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
