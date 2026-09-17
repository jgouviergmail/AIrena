import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { FileText, HeartPulse, Link2, ListMusic, MonitorPlay, Music, Network, Pause, Play, Radio, Theater, Trophy, Users, Volume2, VolumeX } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { RightPanel, type RightPanelTab } from "@/components/layout/RightPanel";
import { ResizeDivider } from "@/components/layout/ResizeDivider";
import { DiscussionFeed } from "@/components/discussion/DiscussionFeed";
import { DiscussionControls } from "@/components/discussion/DiscussionControls";
import { UserInputArea } from "@/components/discussion/UserInputArea";
import { AudienceVote } from "@/components/discussion/AudienceVote";
import { TurnIndicator } from "@/components/discussion/TurnIndicator";
import { SpeakerQueue } from "@/components/discussion/SpeakerQueue";
import { UsagePill } from "@/components/discussion/UsagePill";
import { EmotionPanel } from "@/components/emotion/EmotionSidebar";
import { DocumentPanel } from "@/components/document/DocumentSidebar";
import { ArgumentMapPanel } from "@/components/mindmap/MindmapSidebar";
import { RelationsGraph } from "@/components/relations/RelationsGraph";
import { SourcesPanel } from "@/components/sources/SourcesPanel";
import { ArenaStage } from "@/components/stage/ArenaStage";
import { Scoreboard } from "@/components/stage/Scoreboard";
import { SceneBanner } from "@/components/stage/SceneBanner";
import { TimelineBar } from "@/components/stage/TimelineBar";
import { useArenaShortcuts } from "@/hooks/useArenaShortcuts";
import { speech, useArenaAudio } from "@/hooks/useArenaAudio";
import { setStepMode } from "@/lib/tauri-api";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { setPresentation } from "@/lib/presentation";
import { useUiStore } from "@/stores/useUiStore";
import { cn } from "@/lib/utils";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { toast } from "@/stores/useToastStore";
import { formatUsd } from "@/lib/cost-estimate";
import { LG_BREAKPOINT_QUERY, useMediaQuery } from "@/hooks/useMediaQuery";

const PANEL_MIN_WIDTH = 260;
/** The feed keeps at least this width: the panel may take everything else. */
const FEED_MIN_WIDTH = 320;
const PANEL_DEFAULT_WIDTH = 340;

/** Widest the panel may be right now (the window minus the feed's minimum). */
const panelMaxWidth = () => Math.max(PANEL_MIN_WIDTH, window.innerWidth - FEED_MIN_WIDTH);
const PANEL_WIDTH_STORAGE_KEY = "airena.rightPanel.width";

const tabBadge = (n: number) =>
  n > 0 ? <span className="rounded-full bg-background/60 px-1.5 text-[9px] font-semibold">{n}</span> : undefined;

function readPanelWidth(): number {
  try {
    const stored = Number(localStorage.getItem(PANEL_WIDTH_STORAGE_KEY));
    if (stored >= PANEL_MIN_WIDTH) return Math.min(stored, panelMaxWidth());
  } catch { /* ignore */ }
  return PANEL_DEFAULT_WIDTH;
}

/** Isolated synthesis streaming display — owns its own Zustand subscription so
 *  60ms token flushes only re-render this subtree. Auto-scrolls to keep the
 *  latest tokens visible as the synthesis grows beyond the visible area. */
function SynthesisStreamingPanel() {
  const { t } = useTranslation();
  const status = useArenaStore((s) => s.status);
  const synthesisStreaming = useArenaStore((s) => s.synthesisStreaming);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [synthesisStreaming]);

  if (status !== "synthesizing" && !synthesisStreaming) return null;

  return (
    <div className="flex min-h-0 shrink flex-col border-t border-border bg-card">
      <p className="shrink-0 px-4 pt-3 pb-1 text-xs font-medium text-primary">
        {t("arena.synthesizing")}
      </p>
      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto px-4 pb-3">
        <p className="whitespace-pre-wrap text-sm text-foreground">
          {synthesisStreaming}
          <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
        </p>
      </div>
    </div>
  );
}

export default function ArenaPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const status = useArenaStore((s) => s.status);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const userTurnActive = useArenaStore((s) => s.userTurnActive);
  const voteRequest = useArenaStore((s) => s.voteRequest);
  const determiningOrder = useArenaStore((s) => s.determiningOrder);
  const webSearchCount = useArenaStore((s) => s.webSearchCount);
  const activityStatus = useArenaStore((s) => s.activityStatus);
  const roomMood = useArenaStore((s) => s.roomMood);
  const currentAct = useArenaStore((s) => s.currentAct);
  const error = useArenaStore((s) => s.error);
  const budgetAlert = useArenaStore((s) => s.budgetAlert);
  const thesesCount = useArenaStore((s) => s.argumentMapThesesCount);
  const relationshipsCount = useArenaStore((s) => s.relationships.length);
  const sourcesCount = useArenaStore((s) => s.sources.length);
  const timeline = useArenaStore((s) => s.timeline);
  const stageVisible = useUiStore((s) => s.stageVisible);
  const toggleStage = useUiStore((s) => s.toggleStage);
  const presentation = useUiStore((s) => s.presentationMode);
  const userTimeout = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const prevStatusRef = useRef(status);
  const [panelWidth, setPanelWidth] = useState(readPanelWidth);
  const isDesktop = useMediaQuery(LG_BREAKPOINT_QUERY);
  const hasDocument = documentFormat !== "none";
  useArenaShortcuts(isDesktop);
  useArenaAudio();
  const ttsEnabled = useSettingsStore((s) => s.settings.ttsEnabled);
  const ttsMode = useSettingsStore((s) => s.settings.ttsMode);
  const soundEnabled = useSettingsStore((s) => s.settings.soundEnabled);
  // Voice in "follow" mode: the engine waits for the audience's cue before each speaker (v1.20.1)
  const stepMode = ttsEnabled && ttsMode === "follow";
  const [voicePaused, setVoicePaused] = useState(speech.paused);
  const toggleVoicePause = () => {
    if (speech.paused) speech.resume();
    else speech.pause();
    setVoicePaused(speech.paused);
  };
  useEffect(() => {
    if (status !== "running" && status !== "paused") return;
    setStepMode(stepMode).catch(() => { /* no discussion any more */ });
  }, [stepMode, status]);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  // Leaving the arena leaves the projection too
  useEffect(() => () => { if (useUiStore.getState().presentationMode) void setPresentation(false); }, []);

  const handleResize = useCallback((delta: number) => {
    setPanelWidth((w) => {
      const next = Math.max(PANEL_MIN_WIDTH, Math.min(panelMaxWidth(), w + delta));
      try {
        localStorage.setItem(PANEL_WIDTH_STORAGE_KEY, String(next));
      } catch { /* ignore */ }
      return next;
    });
  }, []);

  // Navigate to summary only when status TRANSITIONS to "ended"
  // (prevents redirect loop on back-button navigation)
  useEffect(() => {
    if (status === "ended" && prevStatusRef.current !== "ended") {
      navigate("/summary");
    }
    prevStatusRef.current = status;
  }, [status, navigate]);

  // Redirect on mount: idle → home, ended → summary
  useEffect(() => {
    if (status === "idle") navigate("/");
    else if (status === "ended") navigate("/summary");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Budget alerts are one-shot engine events → toast once each
  useEffect(() => {
    if (!budgetAlert) return;
    const detail = t("arena.budget.detail", { spent: formatUsd(budgetAlert.spentUsd), budget: formatUsd(budgetAlert.budgetUsd) });
    if (budgetAlert.level === "exceeded") {
      toast.error(t("arena.budget.exceeded"), detail);
    } else {
      toast.warning(t("arena.budget.warning"), detail);
    }
  }, [budgetAlert, t]);

  const tabs = useMemo<RightPanelTab[]>(() => {
    const list: RightPanelTab[] = [
      { id: "emotions", label: t("emotions.sidebar.title"), icon: HeartPulse, content: <EmotionPanel /> },
    ];
    if (hasDocument) {
      list.push({ id: "document", label: t("document.title"), icon: FileText, content: <DocumentPanel /> });
    }
    if (argumentMapEnabled) {
      list.push({ id: "argmap", label: t("mindmap.title"), icon: Network, badge: tabBadge(thesesCount), content: <ArgumentMapPanel /> });
    }
    list.push({ id: "relations", label: t("relations.title"), icon: Users, badge: tabBadge(relationshipsCount), content: <RelationsGraph /> });
    list.push({ id: "score", label: t("score.title"), icon: Trophy, content: <Scoreboard /> });
    if (sourcesCount > 0) {
      list.push({ id: "sources", label: t("sources.title"), icon: Link2, badge: tabBadge(sourcesCount), content: <SourcesPanel /> });
    }
    return list;
  }, [t, hasDocument, argumentMapEnabled, thesesCount, relationshipsCount, sourcesCount]);

  return (
    <>
      <TopBar
        title={`${t("arena.title")} — ${t(`setup.mode_${discussionMode}`)}`}
        subtitle={activityStatus ? (
          <span className="flex items-center gap-1.5 truncate text-xs text-muted-foreground">
            <span className={`h-1.5 w-1.5 shrink-0 animate-pulse rounded-full ${activityStatus.type === "synthesis" ? "bg-blue-500" : "bg-primary"}`} />
            {t(`arena.activity.${activityStatus.type}`, { name: activityStatus.speakerName })}
          </span>
        ) : undefined}
      >
        <button
          type="button"
          onClick={() => updateSettings({ ttsEnabled: !ttsEnabled })}
          aria-pressed={ttsEnabled}
          title={t("stage.voice")}
          className={cn("flex h-8 w-8 items-center justify-center rounded-md transition-colors hover:bg-accent", ttsEnabled ? "text-primary" : "text-muted-foreground")}
        >
          {ttsEnabled ? <Volume2 className="h-4 w-4" /> : <VolumeX className="h-4 w-4" />}
        </button>
        {ttsEnabled && (
          <>
            <button
              type="button"
              onClick={toggleVoicePause}
              aria-pressed={voicePaused}
              title={voicePaused ? t("stage.voiceResume") : t("stage.voicePause")}
              className={cn("flex h-8 w-8 items-center justify-center rounded-md transition-colors hover:bg-accent", voicePaused ? "text-primary" : "text-muted-foreground")}
            >
              {voicePaused ? <Play className="h-4 w-4" /> : <Pause className="h-4 w-4" />}
            </button>
            <button
              type="button"
              onClick={() => updateSettings({ ttsMode: ttsMode === "follow" ? "full" : "follow" })}
              title={ttsMode === "follow" ? t("stage.voiceModeFollow") : t("stage.voiceModeFull")}
              className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent"
            >
              {ttsMode === "follow" ? <Radio className="h-4 w-4" /> : <ListMusic className="h-4 w-4" />}
            </button>
          </>
        )}
        <button
          type="button"
          onClick={() => updateSettings({ soundEnabled: !soundEnabled })}
          aria-pressed={soundEnabled}
          title={t("stage.sounds")}
          className={cn("flex h-8 w-8 items-center justify-center rounded-md transition-colors hover:bg-accent", soundEnabled ? "text-primary" : "text-muted-foreground")}
        >
          <Music className="h-4 w-4" />
        </button>
        <button
          type="button"
          onClick={toggleStage}
          aria-pressed={stageVisible}
          title={t("stage.toggle")}
          className={cn("flex h-8 w-8 items-center justify-center rounded-md transition-colors hover:bg-accent", stageVisible ? "text-primary" : "text-muted-foreground")}
        >
          <Theater className="h-4 w-4" />
        </button>
        <button
          type="button"
          onClick={() => void setPresentation(!presentation)}
          aria-pressed={presentation}
          title={t("stage.projection")}
          className={cn("flex h-8 w-8 items-center justify-center rounded-md transition-colors hover:bg-accent", presentation ? "text-primary" : "text-muted-foreground")}
        >
          <MonitorPlay className="h-4 w-4" />
        </button>
      </TopBar>

      <div className="flex min-h-0 flex-1 flex-col">
        {/* Status bar */}
        <div className="relative z-10 flex flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2 sm:px-4">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <TurnIndicator turn={currentTurn} status={status} determiningOrder={determiningOrder} webSearchCount={webSearchCount} roomMood={roomMood?.label} actTitle={currentAct?.title} />
            <UsagePill />
            <SpeakerQueue />
          </div>
          <DiscussionControls status={status} userTurnActive={userTurnActive} />
        </div>

        {/* Main content + panel */}
        <div className="flex min-h-0 flex-1 overflow-hidden">
          {/* Scrollable content area */}
          <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
            <SceneBanner />
            {stageVisible && <ArenaStage />}
            {error && (
              <div className="border-b border-destructive/30 bg-destructive/5 px-4 py-2 text-sm text-destructive">
                {error}
              </div>
            )}
            {budgetAlert && (
              <div className={`border-b px-4 py-1.5 text-xs ${budgetAlert.level === "exceeded" ? "border-destructive/30 bg-destructive/5 text-destructive" : "border-amber-500/30 bg-amber-500/5 text-amber-600"}`}>
                {budgetAlert.level === "exceeded" ? t("arena.budget.exceeded") : t("arena.budget.warning")}
                {" — "}
                {t("arena.budget.detail", { spent: formatUsd(budgetAlert.spentUsd), budget: formatUsd(budgetAlert.budgetUsd) })}
              </div>
            )}

            <DiscussionFeed />

            <TimelineBar timeline={timeline} turns={currentTurn} currentTurn={currentTurn} />

            <SynthesisStreamingPanel />

            {voteRequest && <AudienceVote phase={voteRequest.phase} timeoutSecs={voteRequest.timeoutSecs} />}

            {userTurnActive && (
              <div className="border-t border-border p-3 sm:p-4">
                <UserInputArea timeoutSecs={userTimeout} />
              </div>
            )}
          </div>

          {isDesktop && !presentation && <ResizeDivider onResize={handleResize} />}
          {!presentation && <RightPanel tabs={tabs} width={panelWidth} />}
        </div>
      </div>
    </>
  );
}
