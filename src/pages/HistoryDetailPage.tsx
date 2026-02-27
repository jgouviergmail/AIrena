import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Bot, Calendar, Download, Network, Repeat, Trash2, Users } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { SimpleMd } from "@/components/shared/SimpleMd";
import { StatCard } from "@/components/shared/StatCard";
import { MarkmapViewer } from "@/components/mindmap/MarkmapViewer";
import type { MarkmapViewerHandle } from "@/components/mindmap/MarkmapViewer";
import { getDiscussionHistory, deleteDiscussionHistory, downloadTextFile, downloadMultipleTextFiles } from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import type { DiscussionDetail } from "@/lib/types";

export default function HistoryDetailPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [detail, setDetail] = useState<DiscussionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<"synthesis" | "discussion" | "argumentMap" | "argumentMapBySpeaker">("synthesis");
  const markmapRef = useRef<MarkmapViewerHandle>(null);
  const markmapBySpeakerRef = useRef<MarkmapViewerHandle>(null);

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
              <StatCard
                label={t("summary.turns")}
                value={String(detail.totalTurns)}
                icon={<Repeat className="h-3.5 w-3.5 text-muted-foreground" />}
              />
              <StatCard
                label={t("summary.participantsList")}
                value={String(detail.participants.filter((p) => p.role !== "user").length)}
                icon={<Users className="h-3.5 w-3.5 text-muted-foreground" />}
              />
              <StatCard
                label={t("summary.model")}
                value={detail.modelName || "-"}
                icon={<Bot className="h-3.5 w-3.5 text-muted-foreground" />}
                truncate
              />
              <StatCard
                label={t("summary.date")}
                value={formatDate(detail.createdAt)}
                icon={<Calendar className="h-3.5 w-3.5 text-muted-foreground" />}
                truncate
              />
            </div>

            {/* Participants with emojis */}
            <div className="rounded-xl border border-border bg-card p-4">
              <p className="mb-2 text-xs text-muted-foreground">{t("summary.participantsList")}</p>
              <div className="flex flex-wrap gap-3">
                {detail.participants.map((p) => (
                  <div key={p.id} className="flex items-center gap-1.5 text-sm text-foreground">
                    <span>{p.emoji}</span>
                    <span className="font-medium">{p.name}</span>
                    {p.role === "IArbitre" && (
                      <span className="rounded bg-primary/10 px-1 text-[9px] font-medium text-primary">IArbitre</span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Tab toggle — narrow */}
          <div className="mx-auto max-w-2xl space-y-1.5">
            <div className="flex rounded-lg border border-border bg-card p-1">
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
            </div>
            {detail.argumentMapMd && (
              <div className="flex rounded-lg border border-border bg-card p-1">
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
                {detail.argumentMapMdBySpeaker && (
                  <button
                    onClick={() => setTab("argumentMapBySpeaker")}
                    className={cn(
                      "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                      tab === "argumentMapBySpeaker"
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    <Users className="mr-1.5 inline h-3.5 w-3.5" />
                    {t("summary.tabArgumentMapBySpeaker")}
                  </button>
                )}
              </div>
            )}
          </div>

          {/* Tab content */}
          {tab === "synthesis" && (
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
          )}
          {tab === "discussion" && (
            <ReadOnlyFeed messages={detail.messages} participants={detail.participants} />
          )}
          {tab === "argumentMap" && (
            <div className="rounded-xl border border-border bg-card p-2" style={{ height: 400 }}>
              <MarkmapViewer ref={markmapRef} markdown={detail.argumentMapMd} />
            </div>
          )}
          {tab === "argumentMapBySpeaker" && (
            <div className="rounded-xl border border-border bg-card p-2" style={{ height: 400 }}>
              <MarkmapViewer ref={markmapBySpeakerRef} markdown={detail.argumentMapMdBySpeaker} />
            </div>
          )}
          {/* Off-screen MarkmapViewers — keep refs alive for SVG export from any tab */}
          {detail.argumentMapMd && tab !== "argumentMap" && (
            <div
              aria-hidden="true"
              style={{ position: "absolute", left: "-9999px", width: "1200px", height: "800px" }}
            >
              <MarkmapViewer ref={markmapRef} markdown={detail.argumentMapMd} />
            </div>
          )}
          {detail.argumentMapMdBySpeaker && tab !== "argumentMapBySpeaker" && (
            <div
              aria-hidden="true"
              style={{ position: "absolute", left: "-9999px", width: "1200px", height: "800px" }}
            >
              <MarkmapViewer ref={markmapBySpeakerRef} markdown={detail.argumentMapMdBySpeaker} />
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
                  onClick={() => downloadMultipleTextFiles([
                    { content: detail.argumentMapMd, fileName: "AIrena - Carte des arguments.md" },
                    ...(detail.argumentMapMdBySpeaker
                      ? [{ content: detail.argumentMapMdBySpeaker, fileName: "AIrena - Carte des arguments par gladiateurs.md" }]
                      : []),
                  ]).catch((e) => console.error("Download failed:", e))}
                  className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  <Download className="h-4 w-4" />
                  {t("summary.downloadArgumentMap")} (.md)
                </button>
                <button
                  onClick={() => {
                    const files: { content: string; fileName: string }[] = [];
                    const svgThesis = markmapRef.current?.getSvgHtml();
                    if (svgThesis) files.push({ content: svgThesis, fileName: "AIrena - Carte des arguments.svg" });
                    const svgSpeaker = markmapBySpeakerRef.current?.getSvgHtml();
                    if (svgSpeaker) files.push({ content: svgSpeaker, fileName: "AIrena - Carte des arguments par gladiateurs.svg" });
                    if (files.length > 0) {
                      downloadMultipleTextFiles(files).catch((e) => console.error("SVG download failed:", e));
                    }
                  }}
                  className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  <Download className="h-4 w-4" />
                  {t("summary.downloadArgumentMapSvg")} (.svg)
                </button>
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
