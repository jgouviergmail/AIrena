import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n/config";
import { Bot, Calendar, Download, FileCode, Link2, Network, Play, Printer, Repeat, TrendingUp, Users } from "lucide-react";
import { renderDiscussionHtml } from "@/lib/export-html";
import { SourcesList } from "@/components/sources/SourcesList";
import { PositionsTable } from "./PositionsTable";
import { AgendaCards } from "./AgendaCards";
import { OutcomePanel } from "./OutcomePanel";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { AwardsCredits } from "./AwardsCredits";
import { ReplayPlayer } from "./ReplayPlayer";
import { sourcesToMarkdown } from "@/lib/sources";
import type { LucideIcon } from "lucide-react";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { SimpleMd } from "@/components/shared/SimpleMd";
import { StatCard } from "@/components/shared/StatCard";
import { UsageSummaryCard } from "@/components/shared/UsageSummaryCard";
import { ArgumentMapStats } from "@/components/mindmap/ArgumentMapStats";
import { MarkmapViewer, type MarkmapViewerHandle } from "@/components/mindmap/MarkmapViewer";
import { downloadMultipleTextFiles, downloadTextFile } from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { logger } from "@/lib/logger";
import { toast } from "@/stores/useToastStore";
import { cn } from "@/lib/utils";
import type { DiscussionReport } from "@/lib/report";
import type { ArgumentMap, LlmUsage, Message, ParticipantInfo } from "@/lib/types";

/** Everything a finished discussion has to show (live summary or history). */
export interface DiscussionReportData {
  topic: string;
  discussionMode: string;
  turns: number;
  modelName: string;
  /** ISO date shown in history (absent on the live summary) */
  createdAt?: string;
  participants: ParticipantInfo[];
  usage: { provider: string; model?: string; total: LlmUsage; calls: number; estimatedCostUsd: number | null } | null;
  synthesis: string;
  messages: Message[];
  argumentMapMd: string;
  argumentMapMdBySpeaker: string;
  argumentMap: ArgumentMap | null;
  documentContent: string;
  documentFormat: string;
  report: DiscussionReport;
}

type Tab = "synthesis" | "discussion" | "replay" | "argumentMap" | "argumentMapBySpeaker" | "sources" | "positions";

const ARGMAP_HEIGHT = 400;
const MAP_FILES = { thesis: "AIrena - Carte des arguments", speaker: "AIrena - Carte des arguments par gladiateurs" };

/** An export that could not be written: logged and told to the user (a cancelled dialog never rejects). */
function reportExportFailure(e: unknown) {
  const message = extractErrorMessage(e);
  logger.error("report", `Export failed: ${message}`);
  toast.error(i18n.t("summary.exportFailed"), message);
}

function TabButton({ active, onClick, icon: Icon, children }: { active: boolean; onClick: () => void; icon?: LucideIcon; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
        active ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground",
      )}
    >
      {Icon && <Icon className="mr-1.5 inline h-3.5 w-3.5" />}
      {children}
    </button>
  );
}

function ActionButton({ onClick, children, icon: Icon = Download }: { onClick: () => void; children: React.ReactNode; icon?: LucideIcon }) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
    >
      <Icon className="h-4 w-4" />
      {children}
    </button>
  );
}

/** Off-screen map kept mounted so its SVG can be exported from any tab (needs real dimensions). */
function HiddenMap({ viewerRef, markdown }: { viewerRef: React.RefObject<MarkmapViewerHandle | null>; markdown: string }) {
  return (
    <div aria-hidden="true" style={{ position: "absolute", left: "-9999px", width: "1200px", height: "800px" }}>
      <MarkmapViewer ref={viewerRef} markdown={markdown} />
    </div>
  );
}

/**
 * Shared report of a finished discussion: stats, usage, participants, tabbed
 * content (synthesis / discussion / argument maps), downloads. Pages add
 * their own navigation through `actions`.
 */
export function DiscussionReportView({ data, actions }: { data: DiscussionReportData; actions: React.ReactNode }) {
  const { t, i18n } = useTranslation();
  const [tab, setTab] = useState<Tab>("synthesis");
  const markmapRef = useRef<MarkmapViewerHandle>(null);
  const markmapBySpeakerRef = useRef<MarkmapViewerHandle>(null);
  const gladiateursCount = useMemo(() => data.participants.filter((p) => p.role === "GladIAteur").length, [data.participants]);
  const hasMap = data.argumentMapMd.length > 0;
  const hasDocument = data.documentContent.length > 0 && data.documentFormat !== "none";
  const sources = data.report.sources;
  const hasSources = sources.length > 0;
  const positions = data.report.positions;
  const agendas = data.report.agendas;
  const hasPositions = positions.length > 0 || agendas.length > 0;

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  // Standalone HTML (offline, printable) with the maps embedded as SVG when rendered
  const exportHtml = () => {
    const html = renderDiscussionHtml({
      data,
      lang: i18n.language,
      svgByThesis: markmapRef.current?.getSvgHtml() ?? null,
      svgBySpeaker: markmapBySpeakerRef.current?.getSvgHtml() ?? null,
      labels: {
        appName: "AIrena", synthesis: t("summary.tabSynthesis"), discussion: t("summary.tabDiscussion"), participants: t("summary.participants"),
        turn: t("sources.turnLabel"), turns: t("summary.turns"), model: t("summary.model"), date: t("summary.date"), sources: t("sources.title"), cited: t("sources.cited"),
        positions: t("positions.title"), agendas: t("agendas.title"), agendaObjective: t("agendas.objective"), agendaRedLine: t("agendas.redLine"), agendaVictory: t("agendas.victory"),
        achieved: t("agendas.achieved"), notAchieved: t("agendas.notAchieved"), unknown: t("agendas.unknown"), outcome: t("summary.outcome"),
        outcomeVerdict: t("outcome.verdictTitle"), outcomeAgreement: t("outcome.agreementTitle"), outcomeVote: t("outcome.swingTitle"),
        argumentMap: t("summary.tabArgumentMap"), argumentMapBySpeaker: t("summary.tabArgumentMapBySpeaker"), document: t("summary.downloadDocument"), reactions: t("score.reactions"),
      },
    });
    downloadTextFile(html, "AIrena - Discussion.html").catch(reportExportFailure);
  };

  const downloadMaps = () =>
    downloadMultipleTextFiles([
      { content: data.argumentMapMd, fileName: `${MAP_FILES.thesis}.md` },
      ...(data.argumentMapMdBySpeaker ? [{ content: data.argumentMapMdBySpeaker, fileName: `${MAP_FILES.speaker}.md` }] : []),
    ]).catch(reportExportFailure);

  const downloadSvgs = () => {
    const files: { content: string; fileName: string }[] = [];
    const svgThesis = markmapRef.current?.getSvgHtml();
    if (svgThesis) files.push({ content: svgThesis, fileName: `${MAP_FILES.thesis}.svg` });
    const svgSpeaker = markmapBySpeakerRef.current?.getSvgHtml();
    if (svgSpeaker) files.push({ content: svgSpeaker, fileName: `${MAP_FILES.speaker}.svg` });
    if (files.length > 0) downloadMultipleTextFiles(files).catch(reportExportFailure);
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      {/* Stats — narrow */}
      <div className="mx-auto max-w-2xl space-y-3">
        <div className="rounded-xl border border-border bg-card p-4">
          <p className="text-xs text-muted-foreground">{t("summary.topic")}</p>
          <p className="mt-1 text-base font-semibold text-foreground">{data.topic || "-"}</p>
          {data.discussionMode && data.discussionMode !== "debate" && (
            <span className="mt-1 inline-block rounded bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
              {t(`setup.mode_${data.discussionMode}`)}
            </span>
          )}
        </div>

        <div className={cn("grid grid-cols-2 gap-3", data.createdAt ? "sm:grid-cols-4" : "sm:grid-cols-3")}>
          <StatCard label={t("summary.turns")} value={String(data.turns)} icon={<Repeat className="h-3.5 w-3.5 text-muted-foreground" />} />
          <StatCard label={t("summary.participantsList")} value={String(gladiateursCount)} icon={<Users className="h-3.5 w-3.5 text-muted-foreground" />} />
          <StatCard label={t("summary.model")} value={data.modelName || "-"} icon={<Bot className="h-3.5 w-3.5 text-muted-foreground" />} truncate />
          {data.createdAt && (
            <StatCard label={t("summary.date")} value={formatDate(data.createdAt)} icon={<Calendar className="h-3.5 w-3.5 text-muted-foreground" />} truncate />
          )}
        </div>

        {data.usage && (
          <UsageSummaryCard
            provider={data.usage.provider}
            model={data.usage.model}
            total={data.usage.total}
            calls={data.usage.calls}
            estimatedCostUsd={data.usage.estimatedCostUsd}
          />
        )}

        <AwardsCredits awards={data.report.awards} participants={data.participants} />

        <DiagnosticsPanel diagnostics={data.report.diagnostics} timings={data.report.timings} />

        <div className="rounded-xl border border-border bg-card p-4">
          <p className="mb-2 text-xs text-muted-foreground">{t("summary.participantsList")}</p>
          <div className="flex flex-wrap gap-3">
            {data.participants.map((p) => (
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

      {/* Tabs — narrow */}
      <div className="mx-auto max-w-2xl space-y-1.5">
        <div className="flex rounded-lg border border-border bg-card p-1">
          <TabButton active={tab === "synthesis"} onClick={() => setTab("synthesis")}>{t("summary.tabSynthesis")}</TabButton>
          <TabButton active={tab === "discussion"} onClick={() => setTab("discussion")}>{t("summary.tabDiscussion")}</TabButton>
          {data.messages.length > 0 && (
            <TabButton active={tab === "replay"} onClick={() => setTab("replay")} icon={Play}>{t("summary.tabReplay")}</TabButton>
          )}
        </div>
        {hasMap && (
          <div className="flex rounded-lg border border-border bg-card p-1">
            <TabButton active={tab === "argumentMap"} onClick={() => setTab("argumentMap")} icon={Network}>{t("summary.tabArgumentMap")}</TabButton>
            {data.argumentMapMdBySpeaker && (
              <TabButton active={tab === "argumentMapBySpeaker"} onClick={() => setTab("argumentMapBySpeaker")} icon={Users}>
                {t("summary.tabArgumentMapBySpeaker")}
              </TabButton>
            )}
          </div>
        )}
        {(hasSources || hasPositions) && (
          <div className="flex rounded-lg border border-border bg-card p-1">
            {hasPositions && (
              <TabButton active={tab === "positions"} onClick={() => setTab("positions")} icon={TrendingUp}>
                {t("summary.tabPositions")}
              </TabButton>
            )}
            {hasSources && (
              <TabButton active={tab === "sources"} onClick={() => setTab("sources")} icon={Link2}>
                {t("summary.tabSources", { count: sources.length })}
              </TabButton>
            )}
          </div>
        )}
      </div>

      {/* Tab content — full width (tables in the synthesis) */}
      {tab === "synthesis" && data.report.outcome && <OutcomePanel outcome={data.report.outcome} />}
      {tab === "synthesis" && (
        <div className="rounded-xl border border-border bg-card p-6">
          <h2 className="mb-4 text-lg font-semibold text-foreground">{t("summary.tabSynthesis")}</h2>
          {data.synthesis ? <SimpleMd text={data.synthesis} /> : <p className="text-sm text-muted-foreground">{t("summary.noSynthesis")}</p>}
        </div>
      )}
      {tab === "discussion" && <ReadOnlyFeed messages={data.messages} participants={data.participants} sources={sources} />}
      {tab === "replay" && <ReplayPlayer messages={data.messages} participants={data.participants} report={data.report} />}
      {tab === "sources" && (
        <div className="flex max-h-[70vh] flex-col rounded-xl border border-border bg-card">
          <SourcesList sources={sources} />
        </div>
      )}
      {tab === "positions" && (
        <div className="space-y-6 rounded-xl border border-border bg-card p-4 sm:p-6">
          <PositionsTable positions={positions} />
          <AgendaCards agendas={agendas} />
        </div>
      )}
      {tab === "argumentMap" && (
        <div className="rounded-xl border border-border bg-card p-2">
          <ArgumentMapStats map={data.argumentMap} />
          <div style={{ height: ARGMAP_HEIGHT }}>
            <MarkmapViewer ref={markmapRef} markdown={data.argumentMapMd} />
          </div>
        </div>
      )}
      {tab === "argumentMapBySpeaker" && (
        <div className="rounded-xl border border-border bg-card p-2">
          <ArgumentMapStats map={data.argumentMap} />
          <div style={{ height: ARGMAP_HEIGHT }}>
            <MarkmapViewer ref={markmapBySpeakerRef} markdown={data.argumentMapMdBySpeaker} />
          </div>
        </div>
      )}
      {hasMap && tab !== "argumentMap" && <HiddenMap viewerRef={markmapRef} markdown={data.argumentMapMd} />}
      {data.argumentMapMdBySpeaker && tab !== "argumentMapBySpeaker" && (
        <HiddenMap viewerRef={markmapBySpeakerRef} markdown={data.argumentMapMdBySpeaker} />
      )}

      {/* Downloads & actions — narrow */}
      <div className="mx-auto max-w-2xl space-y-6">
        {hasDocument && (
          <div className="flex justify-center">
            <ActionButton onClick={() => downloadTextFile(data.documentContent, `airena-document.${data.documentFormat}`).catch(reportExportFailure)}>
              {t("summary.downloadDocument")} (.{data.documentFormat})
            </ActionButton>
          </div>
        )}
        {hasMap && (
          <div className="flex flex-wrap justify-center gap-3">
            <ActionButton onClick={downloadMaps}>{t("summary.downloadArgumentMap")} (.md)</ActionButton>
            <ActionButton onClick={downloadSvgs}>{t("summary.downloadArgumentMapSvg")} (.svg)</ActionButton>
          </div>
        )}
        {hasSources && (
          <div className="flex justify-center">
            <ActionButton
              onClick={() => downloadTextFile(
                sourcesToMarkdown(data.topic, sources, { title: t("sources.title"), turn: t("sources.turnLabel"), cited: t("sources.cited") }),
                "AIrena - Sources.md",
              ).catch(reportExportFailure)}
            >
              {t("sources.export")} (.md)
            </ActionButton>
          </div>
        )}
        <div className="flex flex-wrap justify-center gap-3 print:hidden">
          <ActionButton onClick={exportHtml} icon={FileCode}>{t("summary.exportHtml")} (.html)</ActionButton>
          <ActionButton onClick={() => window.print()} icon={Printer}>{t("summary.print")}</ActionButton>
        </div>
        <div className="flex flex-wrap justify-center gap-3 sm:gap-4 print:hidden">{actions}</div>
      </div>
    </div>
  );
}
