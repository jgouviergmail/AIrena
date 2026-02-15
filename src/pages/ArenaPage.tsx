import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { TopBar } from "@/components/layout/TopBar";
import { DiscussionFeed } from "@/components/discussion/DiscussionFeed";
import { DiscussionControls } from "@/components/discussion/DiscussionControls";
import { UserInputArea } from "@/components/discussion/UserInputArea";
import { TurnIndicator } from "@/components/discussion/TurnIndicator";
import { EmotionSidebar } from "@/components/emotion/EmotionSidebar";
import { DocumentSidebar } from "@/components/document/DocumentSidebar";
import { MindmapSidebar } from "@/components/mindmap/MindmapSidebar";
import { ResizeDivider } from "@/components/layout/ResizeDivider";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";

/** Isolated synthesis streaming display — owns its own Zustand subscription so
 *  60ms token flushes only re-render this subtree. Auto-scrolls to keep the
 *  latest tokens visible as the synthesis grows beyond the visible area. */
function SynthesisStreamingPanel() {
  const { t } = useTranslation();
  const status = useArenaStore((s) => s.status);
  const synthesisStreaming = useArenaStore((s) => s.synthesisStreaming);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on each token flush so the user always sees new content
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
  const determiningOrder = useArenaStore((s) => s.determiningOrder);
  const webSearchCount = useArenaStore((s) => s.webSearchCount);
  const activityStatus = useArenaStore((s) => s.activityStatus);
  const error = useArenaStore((s) => s.error);
  const userTimeout = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const prevStatusRef = useRef(status);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [docSidebarWidth, setDocSidebarWidth] = useState(350);
  const [mindmapSidebarWidth, setMindmapSidebarWidth] = useState(350);
  const hasDocument = documentFormat !== "none";

  const handleResize = useCallback((delta: number) => {
    setSidebarWidth((w) => Math.max(200, Math.min(500, w + delta)));
  }, []);
  const handleDocResize = useCallback((delta: number) => {
    setDocSidebarWidth((w) => Math.max(200, Math.min(600, w + delta)));
  }, []);
  const handleMindmapResize = useCallback((delta: number) => {
    setMindmapSidebarWidth((w) => Math.max(250, Math.min(600, w + delta)));
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
      />

      <div className="flex min-h-0 flex-1 flex-col">
        {/* Status bar */}
        <div className="relative z-10 flex items-center justify-between border-b border-border px-4 py-2">
          <div className="flex items-center gap-3">
            <TurnIndicator turn={currentTurn} status={status} determiningOrder={determiningOrder} webSearchCount={webSearchCount} />
            {discussionMode !== "debate" && (
              <span className="rounded bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                {t(`setup.mode_${discussionMode}`)}
              </span>
            )}
          </div>
          <DiscussionControls status={status} userTurnActive={userTurnActive} />
        </div>

        {/* Main content + sidebars */}
        <div className="flex min-h-0 flex-1 overflow-hidden">
          {/* Scrollable content area */}
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
            {/* Error banner */}
            {error && (
              <div className="border-b border-destructive/30 bg-destructive/5 px-4 py-2 text-sm text-destructive">
                {error}
              </div>
            )}

            {/* Message feed */}
            <DiscussionFeed />

            {/* Synthesis streaming (isolated component to avoid cascading re-renders) */}
            <SynthesisStreamingPanel />

            {/* User input area (when it's user's turn) */}
            {userTurnActive && (
              <div className="border-t border-border p-4">
                <UserInputArea timeoutSecs={userTimeout} />
              </div>
            )}
          </div>

          {/* Mindmap sidebar (if enabled) */}
          {argumentMapEnabled && (
            <>
              <ResizeDivider onResize={handleMindmapResize} />
              <MindmapSidebar width={mindmapSidebarWidth} />
            </>
          )}

          {/* Document sidebar (if format selected) */}
          {hasDocument && (
            <>
              <ResizeDivider onResize={handleDocResize} />
              <DocumentSidebar width={docSidebarWidth} />
            </>
          )}

          {/* Resize divider + Emotion sidebar */}
          <ResizeDivider onResize={handleResize} />
          <EmotionSidebar width={sidebarWidth} />
        </div>
      </div>
    </>
  );
}
