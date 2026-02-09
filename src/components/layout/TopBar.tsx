import { useTranslation } from "react-i18next";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/providers/ThemeProvider";

export function TopBar({ title, children }: { title: string; children?: React.ReactNode }) {
  const { theme, toggleTheme } = useTheme();
  const { t } = useTranslation();

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-card px-4">
      <h1 className="text-sm font-semibold text-foreground">{title}</h1>

      <div className="flex items-center gap-2">
        {children}
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
