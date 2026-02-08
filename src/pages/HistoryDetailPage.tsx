import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Trash2 } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { getDiscussionHistory, deleteDiscussionHistory } from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import type { DiscussionDetail } from "@/lib/types";

export default function HistoryDetailPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [detail, setDetail] = useState<DiscussionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<"discussion" | "synthesis">("discussion");

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    getDiscussionHistory(id)
      .then(setDetail)
      .catch((err) => console.error("Failed to load discussion:", err))
      .finally(() => setLoading(false));
  }, [id]);

  const handleDelete = () => {
    if (!id || !window.confirm(t("history.deleteConfirm"))) return;
    deleteDiscussionHistory(id)
      .then(() => navigate("/history"))
      .catch((err) => console.error("Failed to delete:", err));
  };

  if (loading) {
    return (
      <>
        <TopBar title={t("history.title")} />
        <div className="flex flex-1 items-center justify-center text-muted-foreground">...</div>
      </>
    );
  }

  if (!detail) {
    return (
      <>
        <TopBar title={t("history.title")} />
        <div className="flex flex-1 items-center justify-center text-muted-foreground">
          {t("history.empty")}
        </div>
      </>
    );
  }

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  return (
    <>
      <TopBar title={detail.topic} />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl space-y-6">
          {/* Stats */}
          <div className="grid grid-cols-3 gap-4">
            <div className="rounded-lg border border-border bg-card p-3 text-center">
              <p className="text-xs text-muted-foreground">{t("summary.turns")}</p>
              <p className="mt-1 text-sm font-semibold text-foreground">{detail.totalTurns}</p>
            </div>
            <div className="rounded-lg border border-border bg-card p-3 text-center">
              <p className="text-xs text-muted-foreground">{t("summary.participants")}</p>
              <p className="mt-1 text-sm font-semibold text-foreground">
                {detail.participants.filter((p) => p.role !== "user").length}
              </p>
            </div>
            <div className="rounded-lg border border-border bg-card p-3 text-center">
              <p className="text-xs text-muted-foreground">{t("summary.topic")}</p>
              <p className="mt-1 truncate text-sm font-semibold text-foreground" title={formatDate(detail.createdAt)}>
                {formatDate(detail.createdAt)}
              </p>
            </div>
          </div>

          {/* Tab toggle */}
          <div className="flex rounded-lg border border-border bg-card p-1">
            <button
              onClick={() => setTab("discussion")}
              className={cn(
                "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                tab === "discussion"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t("summary.tabDiscussion")}
            </button>
            <button
              onClick={() => setTab("synthesis")}
              className={cn(
                "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                tab === "synthesis"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t("summary.tabSynthesis")}
            </button>
          </div>

          {/* Tab content */}
          {tab === "discussion" ? (
            <ReadOnlyFeed messages={detail.messages} participants={detail.participants} />
          ) : (
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold text-foreground">
                {t("summary.tabSynthesis")}
              </h2>
              <div className="prose prose-sm max-w-none text-sm text-foreground">
                <p className="whitespace-pre-wrap">
                  {detail.synthesis || t("summary.noSynthesis")}
                </p>
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-center gap-4">
            <button
              onClick={() => navigate("/history")}
              className="flex items-center gap-2 rounded-lg border border-border bg-card px-5 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("history.back")}
            </button>
            <button
              onClick={handleDelete}
              className="flex items-center gap-2 rounded-lg border border-destructive/30 px-5 py-2.5 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
            >
              <Trash2 className="h-4 w-4" />
              {t("history.delete")}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
