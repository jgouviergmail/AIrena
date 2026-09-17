import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Trash2 } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { DiscussionReportView, type DiscussionReportData } from "@/components/report/DiscussionReportView";
import { parseArgumentMapJson } from "@/components/mindmap/ArgumentMapStats";
import { deleteDiscussionHistory, getDiscussionHistory } from "@/lib/tauri-api";
import { parseReportJson } from "@/lib/report";
import type { DiscussionDetail } from "@/lib/types";

export default function HistoryDetailPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [detail, setDetail] = useState<DiscussionDetail | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    getDiscussionHistory(id)
      .then(setDetail)
      .catch((err) => console.error("Failed to load discussion:", err))
      .finally(() => setLoading(false));
  }, [id]);

  // Structured map (v1.16+) and report (v1.17+): older discussions only carry the markdown views
  const data = useMemo<DiscussionReportData | null>(() => {
    if (!detail) return null;
    return {
      topic: detail.topic,
      discussionMode: detail.discussionMode,
      turns: detail.totalTurns,
      modelName: detail.modelName,
      createdAt: detail.createdAt,
      participants: detail.participants,
      usage: { provider: detail.llmProvider, total: detail.usage.total, calls: detail.usage.calls, estimatedCostUsd: detail.llmProvider === "ollama" ? null : detail.estimatedCostUsd },
      synthesis: detail.synthesis,
      messages: detail.messages,
      argumentMapMd: detail.argumentMapMd,
      argumentMapMdBySpeaker: detail.argumentMapMdBySpeaker,
      argumentMap: parseArgumentMapJson(detail.argumentMapJson),
      documentContent: detail.documentContent,
      documentFormat: detail.documentFormat,
      report: parseReportJson(detail.reportJson),
    };
  }, [detail]);

  const handleDelete = () => {
    if (!id || !window.confirm(t("history.deleteConfirm"))) return;
    deleteDiscussionHistory(id)
      .then(() => navigate("/history"))
      .catch((err) => console.error("Failed to delete:", err));
  };

  if (loading || !data) {
    return (
      <>
        <TopBar title={t("history.title")} />
        <div className="flex flex-1 items-center justify-center text-muted-foreground">
          {loading ? "..." : t("history.empty")}
        </div>
      </>
    );
  }

  return (
    <>
      <TopBar title={data.topic} />
      <div className="flex-1 overflow-y-auto p-4 sm:p-6">
        <DiscussionReportView
          data={data}
          actions={
            <>
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
            </>
          }
        />
      </div>
    </>
  );
}
