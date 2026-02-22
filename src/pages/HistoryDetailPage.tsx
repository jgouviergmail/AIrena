import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Bot, Calendar, Download, Network, Repeat, Trash2, Users } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { SimpleMd } from "@/components/shared/SimpleMd";
import { MarkmapViewer } from "@/components/mindmap/MarkmapViewer";
import type { MarkmapViewerHandle } from "@/components/mindmap/MarkmapViewer";
import { getDiscussionHistory, deleteDiscussionHistory, downloadTextFile } from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import type { DiscussionDetail } from "@/lib/types";

export default function HistoryDetailPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [detail, setDetail] = useState<DiscussionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<"discussion" | "synthesis" | "argumentMap">("discussion");
  const markmapRef = useRef<MarkmapViewerHandle>(null);

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
        <div className="mx-auto max-w-5xl space-y-6">
          {/* Stats — narrow */}
          <div className="mx-auto max-w-2xl space-y-3">
            {/* Topic */}
            <div className="rounded-xl border border-border bg-card p-4">
              <p className="text-xs text-muted-foreground">{t("summary.topic")}</p>
              <p className="mt-1 text-base font-semibold text-foreground">{detail.topic}</p>
              {detail.discussionMode && detail.discussionMode !== "debate" && (
                <span className="mt-1 inline-block rounded bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                  {t(`setup.mode_${detail.discussionMode}`)}
                </span>
              )}
            </div>

            {/* Stats */}
            <div className="grid grid-cols-4 gap-3">
              <div className="rounded-xl border border-border bg-card p-3 text-center">
                <div className="flex items-center justify-center gap-1">
                  <Repeat className="h-3.5 w-3.5 text-muted-foreground" />
                  <p className="text-xs text-muted-foreground">{t("summary.turns")}</p>
                </div>
                <p className="mt-1 text-sm font-semibold text-foreground">{detail.totalTurns}</p>
              </div>
              <div className="rounded-xl border border-border bg-card p-3 text-center">
                <div className="flex items-center justify-center gap-1">
                  <Users className="h-3.5 w-3.5 text-muted-foreground" />
                  <p className="text-xs text-muted-foreground">{t("summary.participantsList")}</p>
                </div>
                <p className="mt-1 text-sm font-semibold text-foreground">
                  {detail.participants.filter((p) => p.role !== "user").length}
                </p>
              </div>
              <div className="rounded-xl border border-border bg-card p-3 text-center">
                <div className="flex items-center justify-center gap-1">
                  <Bot className="h-3.5 w-3.5 text-muted-foreground" />
                  <p className="text-xs text-muted-foreground">{t("summary.model")}</p>
                </div>
                <p className="mt-1 truncate text-sm font-semibold text-foreground" title={detail.modelName}>
                  {detail.modelName || "-"}
                </p>
              </div>
              <div className="rounded-xl border border-border bg-card p-3 text-center">
                <div className="flex items-center justify-center gap-1">
                  <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
                  <p className="text-xs text-muted-foreground">{t("summary.date")}</p>
                </div>
                <p className="mt-1 truncate text-sm font-semibold text-foreground" title={formatDate(detail.createdAt)}>
                  {formatDate(detail.createdAt)}
                </p>
              </div>
            </div>
          </div>

          {/* Tab toggle — narrow */}
          <div className="mx-auto max-w-2xl">
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
              {detail.argumentMapMd && (
                <button
                  onClick={() => setTab("argumentMap")}
                  className={cn(
                    "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                    tab === "argumentMap"
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  <Network className="mr-1.5 inline h-3.5 w-3.5" />
                  {t("summary.tabArgumentMap")}
                </button>
              )}
            </div>
          </div>

          {/* Tab content — full width for tables in synthesis */}
          {tab === "discussion" ? (
            <ReadOnlyFeed messages={detail.messages} participants={detail.participants} />
          ) : tab === "synthesis" ? (
            <div className="rounded-xl border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold text-foreground">
                {t("summary.tabSynthesis")}
              </h2>
              {detail.synthesis ? (
                <SimpleMd text={detail.synthesis} />
              ) : (
                <p className="text-sm text-muted-foreground">{t("summary.noSynthesis")}</p>
              )}
            </div>
          ) : (
            <div className="rounded-xl border border-border bg-card p-2" style={{ height: 400 }}>
              <MarkmapViewer ref={markmapRef} markdown={detail.argumentMapMd} />
            </div>
          )}

          {/* Downloads & actions — narrow */}
          <div className="mx-auto max-w-2xl space-y-6">
            {/* Document download */}
            {detail.documentContent && detail.documentFormat && detail.documentFormat !== "none" && (
              <div className="flex justify-center">
                <button
                  onClick={() => downloadTextFile(detail.documentContent, `airena-document.${detail.documentFormat}`).catch((e) => console.error("Download failed:", e))}
                  className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  <Download className="h-4 w-4" />
                  {t("summary.downloadDocument")} (.{detail.documentFormat})
                </button>
              </div>
            )}

            {/* Argument map download */}
            {detail.argumentMapMd && (
              <div className="flex justify-center gap-3">
                <button
                  onClick={() => downloadTextFile(detail.argumentMapMd, "airena-argument-map.md").catch((e) => console.error("Download failed:", e))}
                  className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  <Download className="h-4 w-4" />
                  {t("summary.downloadArgumentMap")} (.md)
                </button>
                {tab === "argumentMap" && (
                  <button
                    onClick={() => {
                      const svgHtml = markmapRef.current?.getSvgHtml();
                      if (svgHtml) {
                        downloadTextFile(svgHtml, "airena-argument-map.svg").catch((e) => console.error("SVG download failed:", e));
                      }
                    }}
                    className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                  >
                    <Download className="h-4 w-4" />
                    {t("summary.downloadArgumentMapSvg")} (.svg)
                  </button>
                )}
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
      </div>
    </>
  );
}
