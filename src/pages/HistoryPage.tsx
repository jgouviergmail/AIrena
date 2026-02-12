import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Trash2 } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import {
  listDiscussionHistory,
  deleteDiscussionHistory,
  deleteAllDiscussionHistory,
} from "@/lib/tauri-api";
import type { DiscussionSummary } from "@/lib/types";

export default function HistoryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [discussions, setDiscussions] = useState<DiscussionSummary[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchHistory = () => {
    setLoading(true);
    listDiscussionHistory()
      .then(setDiscussions)
      .catch((err) => console.error("Failed to load history:", err))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    fetchHistory();
  }, []);

  const handleDelete = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!window.confirm(t("history.deleteConfirm"))) return;
    deleteDiscussionHistory(id)
      .then(fetchHistory)
      .catch((err) => console.error("Failed to delete:", err));
  };

  const handleDeleteAll = () => {
    if (!window.confirm(t("history.deleteAllConfirm"))) return;
    deleteAllDiscussionHistory()
      .then(fetchHistory)
      .catch((err) => console.error("Failed to delete all:", err));
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
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl space-y-4">
          {discussions.length > 0 && (
            <div className="flex justify-end">
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
          ) : discussions.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              {t("history.empty")}
            </div>
          ) : (
            discussions.map((d) => (
              <button
                key={d.id}
                onClick={() => navigate(`/history/${d.id}`)}
                className="flex w-full items-start gap-3 rounded-lg border border-border bg-card p-4 text-left transition-colors hover:bg-accent/50"
              >
                {/* Participant emojis */}
                <div className="flex shrink-0 -space-x-1 pt-0.5">
                  {d.participants.slice(0, 5).map((p) => (
                    <span key={p.id} className="text-lg" title={p.name}>
                      {p.emoji}
                    </span>
                  ))}
                </div>

                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-foreground">
                    {formatDate(d.createdAt)} — {t(`setup.mode_${d.discussionMode}`)} — {d.topic}
                  </p>
                  <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
                    <span>{t("history.turns", { count: d.totalTurns })}</span>
                    <span className="truncate text-[10px]">{d.modelName}</span>
                    {!d.hasSynthesis && (
                      <span className="text-destructive/70">{t("history.noSynthesis")}</span>
                    )}
                  </div>
                </div>

                <button
                  onClick={(e) => handleDelete(d.id, e)}
                  className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                  title={t("history.delete")}
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </button>
            ))
          )}
        </div>
      </div>
    </>
  );
}
