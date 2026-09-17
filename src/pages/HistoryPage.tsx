import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Star, Trash2 } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { HistoryFilters } from "@/components/history/HistoryFilters";
import { TagEditor } from "@/components/history/TagEditor";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { collectTags, EMPTY_FILTERS, filterDiscussions, type HistoryFilters as Filters } from "@/lib/history-filters";
import type { DiscussionSummary } from "@/lib/types";
import { formatTokens, formatUsd } from "@/lib/cost-estimate";
import { cn } from "@/lib/utils";
import { toast } from "@/stores/useToastStore";

/** Debounce before a full-text search hits the backend (ms). */
const SEARCH_DEBOUNCE_MS = 300;

export default function HistoryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [discussions, setDiscussions] = useState<DiscussionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<Filters>(EMPTY_FILTERS);
  const requestSeq = useRef(0);

  // Load (or search) the list; a stale answer never overwrites a newer one
  const fetchHistory = (q: string) => {
    const seq = ++requestSeq.current;
    setLoading(true);
    (q.trim() ? api.searchDiscussionHistory(q) : api.listDiscussionHistory())
      .then((rows) => { if (seq === requestSeq.current) setDiscussions(rows); })
      .catch((e: unknown) => toast.error(t("history.loadError"), extractErrorMessage(e)))
      .finally(() => { if (seq === requestSeq.current) setLoading(false); });
  };

  useEffect(() => {
    const timer = setTimeout(() => fetchHistory(query), query ? SEARCH_DEBOUNCE_MS : 0);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query]);

  const tags = useMemo(() => collectTags(discussions), [discussions]);
  const visible = useMemo(() => filterDiscussions(discussions, filters), [discussions, filters]);

  const patchRow = (id: string, patch: Partial<DiscussionSummary>) =>
    setDiscussions((rows) => rows.map((d) => (d.id === id ? { ...d, ...patch } : d)));

  const handleDelete = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!window.confirm(t("history.deleteConfirm"))) return;
    api.deleteDiscussionHistory(id)
      .then(() => fetchHistory(query))
      .catch((e: unknown) => toast.error(t("history.loadError"), extractErrorMessage(e)));
  };

  const handleDeleteAll = () => {
    if (!window.confirm(t("history.deleteAllConfirm"))) return;
    api.deleteAllDiscussionHistory()
      .then(() => fetchHistory(query))
      .catch((e: unknown) => toast.error(t("history.loadError"), extractErrorMessage(e)));
  };

  const toggleFavorite = (d: DiscussionSummary, e: React.MouseEvent) => {
    e.stopPropagation();
    const favorite = !d.favorite;
    patchRow(d.id, { favorite });
    api.setDiscussionFavorite(d.id, favorite).catch((err: unknown) => {
      patchRow(d.id, { favorite: d.favorite });
      toast.error(t("history.updateError"), extractErrorMessage(err));
    });
  };

  const setTags = (d: DiscussionSummary, next: string[]) => {
    patchRow(d.id, { tags: next });
    api.setDiscussionTags(d.id, next).catch((err: unknown) => {
      patchRow(d.id, { tags: d.tags });
      toast.error(t("history.updateError"), extractErrorMessage(err));
    });
  };

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  return (
    <>
      <TopBar title={t("history.title")} />
      <div className="flex-1 overflow-y-auto p-4 sm:p-6">
        <div className="mx-auto max-w-2xl space-y-4">
          <HistoryFilters query={query} onQuery={setQuery} filters={filters} onFilters={setFilters} tags={tags} />

          {discussions.length > 0 && (
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs text-muted-foreground">{t("history.count", { shown: visible.length, total: discussions.length })}</span>
              <button
                onClick={handleDeleteAll}
                className="flex items-center gap-1.5 rounded-lg border border-destructive/30 px-3 py-1.5 text-xs font-medium text-destructive transition-colors hover:bg-destructive/10"
              >
                <Trash2 className="h-3 w-3" />
                {t("history.deleteAll")}
              </button>
            </div>
          )}

          {loading ? (
            <div className="py-12 text-center text-muted-foreground">...</div>
          ) : visible.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              {discussions.length === 0 && !query ? t("history.empty") : t("history.noMatch")}
            </div>
          ) : (
            visible.map((d) => (
              <div
                key={d.id}
                role="button"
                tabIndex={0}
                onClick={() => navigate(`/history/${d.id}`)}
                onKeyDown={(e) => { if (e.key === "Enter") navigate(`/history/${d.id}`); }}
                className="flex w-full cursor-pointer items-start gap-3 rounded-lg border border-border bg-card p-4 text-left transition-colors hover:bg-accent/50"
              >
                {/* Participant emojis */}
                <div className="flex shrink-0 -space-x-1 pt-0.5">
                  {d.participants.slice(0, 5).map((p) => (
                    <span key={p.id} className="text-lg" title={p.name}>
                      {p.emoji}
                    </span>
                  ))}
                </div>

                <div className="min-w-0 flex-1 space-y-1">
                  <p className="truncate text-sm font-medium text-foreground">
                    {formatDate(d.createdAt)} — {t(`setup.mode_${d.discussionMode}`)} — {d.topic}
                  </p>
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
                    <span>{t("history.turns", { count: d.totalTurns })}</span>
                    <span className="truncate text-[10px]">{d.modelName}</span>
                    {d.totalTokens > 0 && (
                      <span className="font-mono text-[10px]">{formatTokens(d.totalTokens)} tok</span>
                    )}
                    {d.llmProvider !== "ollama" && d.estimatedCostUsd > 0 && (
                      <span className="font-mono text-[10px] text-amber-600 dark:text-amber-400">{formatUsd(d.estimatedCostUsd)}</span>
                    )}
                    {!d.hasSynthesis && (
                      <span className="text-destructive/70">{t("history.noSynthesis")}</span>
                    )}
                  </div>
                  <TagEditor tags={d.tags} onChange={(next) => setTags(d, next)} />
                </div>

                <div className="flex shrink-0 items-center gap-0.5">
                  <button
                    onClick={(e) => toggleFavorite(d, e)}
                    aria-pressed={d.favorite}
                    className={cn("rounded p-1 transition-colors", d.favorite ? "text-amber-500" : "text-muted-foreground hover:bg-accent hover:text-amber-500")}
                    title={d.favorite ? t("history.unfavorite") : t("history.favorite")}
                  >
                    <Star className={cn("h-4 w-4", d.favorite && "fill-current")} />
                  </button>
                  <button
                    onClick={(e) => handleDelete(d.id, e)}
                    className="rounded p-1 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                    title={t("history.delete")}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </>
  );
}
