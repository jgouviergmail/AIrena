import { useTranslation } from "react-i18next";
import { Search, Star, X } from "lucide-react";
import { DISCUSSION_MODES } from "@/lib/modes";
import { EMPTY_FILTERS, hasActiveFilters, type HistoryFilters as Filters } from "@/lib/history-filters";
import { cn } from "@/lib/utils";

const PROVIDERS = ["ollama", "deepseek", "openaiCompat"] as const;
const selectClass = "rounded-md border border-border bg-background px-2 py-1 text-xs text-foreground";

/** Search box + narrowing controls of the history page (state owned by the page). */
export function HistoryFilters({
  query, onQuery, filters, onFilters, tags,
}: {
  query: string;
  onQuery: (q: string) => void;
  filters: Filters;
  onFilters: (f: Filters) => void;
  tags: string[];
}) {
  const { t } = useTranslation();
  const set = (patch: Partial<Filters>) => onFilters({ ...filters, ...patch });
  return (
    <div className="space-y-2">
      <label className="relative block">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
        <input
          type="search"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder={t("history.searchPlaceholder")}
          aria-label={t("history.search")}
          className="w-full rounded-md border border-border bg-background py-2 pl-9 pr-3 text-sm text-foreground"
        />
      </label>
      <div className="flex flex-wrap items-center gap-2">
        <select value={filters.mode ?? ""} onChange={(e) => set({ mode: e.target.value || null })} aria-label={t("history.filterMode")} className={selectClass}>
          <option value="">{t("history.filterMode")}</option>
          {DISCUSSION_MODES.map((m) => <option key={m} value={m}>{t(`setup.mode_${m}`)}</option>)}
        </select>
        <select value={filters.provider ?? ""} onChange={(e) => set({ provider: e.target.value || null })} aria-label={t("history.filterProvider")} className={selectClass}>
          <option value="">{t("history.filterProvider")}</option>
          {PROVIDERS.map((p) => <option key={p} value={p}>{t(`settings.provider_${p}`)}</option>)}
        </select>
        {tags.length > 0 && (
          <select value={filters.tag ?? ""} onChange={(e) => set({ tag: e.target.value || null })} aria-label={t("history.filterTag")} className={selectClass}>
            <option value="">{t("history.filterTag")}</option>
            {tags.map((tag) => <option key={tag} value={tag}>#{tag}</option>)}
          </select>
        )}
        <input
          type="text"
          value={filters.participant}
          onChange={(e) => set({ participant: e.target.value })}
          placeholder={t("history.filterParticipant")}
          aria-label={t("history.filterParticipant")}
          className={cn(selectClass, "w-36")}
        />
        <button
          type="button"
          onClick={() => set({ favoriteOnly: !filters.favoriteOnly })}
          aria-pressed={filters.favoriteOnly}
          className={cn("inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs", filters.favoriteOnly ? "border-amber-500/50 bg-amber-500/10 text-amber-700 dark:text-amber-300" : "border-border text-muted-foreground hover:bg-accent")}
        >
          <Star className={cn("h-3.5 w-3.5", filters.favoriteOnly && "fill-current")} aria-hidden="true" />
          {t("history.favoritesOnly")}
        </button>
        {hasActiveFilters(filters) && (
          <button type="button" onClick={() => onFilters(EMPTY_FILTERS)} className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground">
            <X className="h-3.5 w-3.5" aria-hidden="true" />
            {t("history.clearFilters")}
          </button>
        )}
      </div>
    </div>
  );
}
