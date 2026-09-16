import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight, PanelRightOpen, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { LG_BREAKPOINT_QUERY, useMediaQuery } from "@/hooks/useMediaQuery";

export interface RightPanelTab {
  id: string;
  label: string;
  icon: LucideIcon;
  /** Small counter/badge next to the label */
  badge?: React.ReactNode;
  content: React.ReactNode;
}

const TAB_STORAGE_KEY = "airena.rightPanel.tab";
const COLLAPSED_STORAGE_KEY = "airena.rightPanel.collapsed";

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch { /* private mode / disabled storage */ }
}

/**
 * Tabbed side panel replacing the three stacked sidebars. Desktop: a resizable
 * column (collapsible to a rail). Narrow screens: a floating button opens the
 * panel as a full-height drawer over the feed.
 */
export function RightPanel({ tabs, width }: { tabs: RightPanelTab[]; width: number }) {
  const { t } = useTranslation();
  const [activeId, setActiveId] = useState<string>(() => readStorage(TAB_STORAGE_KEY) ?? tabs[0]?.id ?? "");
  const [collapsed, setCollapsed] = useState<boolean>(() => readStorage(COLLAPSED_STORAGE_KEY) === "1");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const isDesktop = useMediaQuery(LG_BREAKPOINT_QUERY);

  // Fall back to the first tab when the remembered one is not available in this discussion
  const active = tabs.find((tab) => tab.id === activeId) ?? tabs[0];
  useEffect(() => {
    if (active && active.id !== activeId) setActiveId(active.id);
  }, [active, activeId]);

  const selectTab = (id: string) => {
    setActiveId(id);
    writeStorage(TAB_STORAGE_KEY, id);
  };
  const toggleCollapsed = () => {
    setCollapsed((c) => {
      writeStorage(COLLAPSED_STORAGE_KEY, c ? "0" : "1");
      return !c;
    });
  };

  if (tabs.length === 0 || !active) return null;

  const tabBar = (
    <div role="tablist" className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-border px-2 py-1.5">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={tab.id === active.id}
          onClick={() => selectTab(tab.id)}
          className={cn(
            "flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium transition-colors",
            tab.id === active.id
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground",
          )}
        >
          <tab.icon className="h-3.5 w-3.5" />
          {tab.label}
          {tab.badge}
        </button>
      ))}
    </div>
  );

  if (isDesktop) {
    return collapsed ? (
      <aside className="flex w-8 shrink-0 flex-col items-center border-l border-border bg-card/50 pt-2">
        <button
          onClick={toggleCollapsed}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("panel.expand")}
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="mt-2 text-xs text-muted-foreground [writing-mode:vertical-lr]">{active.label}</span>
      </aside>
    ) : (
      <aside className="flex shrink-0 flex-col border-l border-border bg-card/50" style={{ width: `${width}px` }}>
        <div className="flex shrink-0 items-center justify-between border-b border-border pr-1">
          <div className="min-w-0 flex-1">{tabBar}</div>
          <button
            onClick={toggleCollapsed}
            className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
            title={t("panel.collapse")}
          >
            <ChevronRight className="h-4 w-4" />
          </button>
        </div>
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">{active.content}</div>
      </aside>
    );
  }

  // Narrow screens: floating opener + drawer (content mounted only while open)
  return (
    <>
      <button
        onClick={() => setDrawerOpen(true)}
        className="fixed bottom-4 right-4 z-30 flex items-center gap-1.5 rounded-full bg-primary px-3 py-2 text-xs font-medium text-primary-foreground shadow-lg"
        title={t("panel.open")}
      >
        <PanelRightOpen className="h-4 w-4" />
        {active.label}
      </button>
      {drawerOpen && (
        <div className="fixed inset-0 z-40 flex" role="dialog" aria-modal="true">
          <button
            aria-label={t("panel.close")}
            onClick={() => setDrawerOpen(false)}
            className="flex-1 bg-black/40 motion-safe:animate-in motion-safe:fade-in"
          />
          <div className="flex w-[min(92vw,420px)] flex-col border-l border-border bg-card motion-safe:animate-in motion-safe:slide-in-from-right">
            <div className="flex shrink-0 items-center justify-between border-b border-border pr-1">
              <div className="min-w-0 flex-1">{tabBar}</div>
              <button
                onClick={() => setDrawerOpen(false)}
                className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                title={t("panel.close")}
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden">{active.content}</div>
          </div>
        </div>
      )}
    </>
  );
}
