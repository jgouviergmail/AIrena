import { useTranslation } from "react-i18next";
import { Cpu, Moon, Sun } from "lucide-react";
import { useTheme } from "@/providers/ThemeProvider";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";

export function TopBar({ title, subtitle, children }: { title: string; subtitle?: React.ReactNode; children?: React.ReactNode }) {
  const { theme, toggleTheme } = useTheme();
  const { t } = useTranslation();
  const modelBudgetInfo = useSettingsStore((s) => s.modelBudgetInfo);

  const vramPct = modelBudgetInfo?.vram && modelBudgetInfo?.ollamaVramMb != null
    ? Math.round((modelBudgetInfo.ollamaVramMb / modelBudgetInfo.vram.totalMb) * 100)
    : null;

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-card px-4">
      <div className="flex min-w-0 items-center gap-3">
        <h1 className="shrink-0 text-sm font-semibold text-foreground">{title}</h1>
        {subtitle}
      </div>

      <div className="flex items-center gap-2">
        {children}
        {vramPct != null && modelBudgetInfo?.vram && (
          <div
            className="flex items-center gap-1.5"
            title={`VRAM: ${modelBudgetInfo.ollamaVramMb!.toLocaleString()} / ${modelBudgetInfo.vram.totalMb.toLocaleString()} MB`}
          >
            <Cpu className="h-3 w-3 text-muted-foreground" />
            <div className="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
              <div
                className={cn(
                  "h-full rounded-full transition-all",
                  vramPct > 90 ? "bg-destructive" : vramPct > 70 ? "bg-amber-500" : "bg-primary",
                )}
                style={{ width: `${Math.min(100, vramPct)}%` }}
              />
            </div>
            <span className="text-xs tabular-nums text-muted-foreground">{vramPct}%</span>
          </div>
        )}
        <button
          onClick={toggleTheme}
          title={theme === "dark" ? t("settings.themeLight") : t("settings.themeDark")}
          className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          {theme === "dark" ? (
            <Sun className="h-4 w-4" />
          ) : (
            <Moon className="h-4 w-4" />
          )}
        </button>
      </div>
    </header>
  );
}
