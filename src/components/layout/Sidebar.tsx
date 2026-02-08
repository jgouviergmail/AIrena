import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { History, Home, MessageSquarePlus, Settings, Swords } from "lucide-react";
import { cn } from "@/lib/utils";

const navItems = [
  { key: "home", path: "/", icon: Home, labelKey: "nav.home" },
  {
    key: "setup",
    path: "/setup",
    icon: MessageSquarePlus,
    labelKey: "nav.newDiscussion",
  },
  {
    key: "history",
    path: "/history",
    icon: History,
    labelKey: "nav.history",
  },
  {
    key: "settings",
    path: "/settings",
    icon: Settings,
    labelKey: "nav.settings",
  },
] as const;

export function Sidebar() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();

  const isArenaActive =
    location.pathname === "/arena" || location.pathname === "/summary";
  const isHistoryActive = location.pathname.startsWith("/history");

  return (
    <aside className="flex h-full w-16 flex-col items-center border-r border-sidebar-border bg-sidebar py-4">
      <div className="mb-6 flex h-10 w-10 items-center justify-center rounded-lg bg-primary">
        <Swords className="h-5 w-5 text-primary-foreground" />
      </div>

      <nav className="flex flex-1 flex-col items-center gap-2">
        {navItems.map((item) => {
          const isActive = item.key === "history"
            ? isHistoryActive
            : location.pathname === item.path;
          return (
            <button
              key={item.key}
              onClick={() => navigate(item.path)}
              title={t(item.labelKey)}
              className={cn(
                "flex h-10 w-10 items-center justify-center rounded-lg transition-colors",
                isActive
                  ? "bg-sidebar-accent text-sidebar-primary"
                  : "text-sidebar-foreground/60 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
              )}
            >
              <item.icon className="h-5 w-5" />
            </button>
          );
        })}

        {isArenaActive && (
          <button
            title={t("arena.title")}
            className="flex h-10 w-10 items-center justify-center rounded-lg bg-sidebar-accent text-sidebar-primary"
          >
            <Swords className="h-5 w-5" />
          </button>
        )}
      </nav>
    </aside>
  );
}
