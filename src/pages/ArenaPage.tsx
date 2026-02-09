import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { TopBar } from "@/components/layout/TopBar";
import { DiscussionFeed } from "@/components/discussion/DiscussionFeed";
import { DiscussionControls } from "@/components/discussion/DiscussionControls";
import { UserInputArea } from "@/components/discussion/UserInputArea";
import { TurnIndicator } from "@/components/discussion/TurnIndicator";
import { EmotionSidebar } from "@/components/emotion/EmotionSidebar";
import { ResizeDivider } from "@/components/layout/ResizeDivider";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";

export default function ArenaPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const status = useArenaStore((s) => s.status);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const userTurnActive = useArenaStore((s) => s.userTurnActive);
  const determiningOrder = useArenaStore((s) => s.determiningOrder);
  const webSearchCount = useArenaStore((s) => s.webSearchCount);
  const error = useArenaStore((s) => s.error);
  const synthesisStreaming = useArenaStore((s) => s.synthesisStreaming);
  const userTimeout = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const prevStatusRef = useRef(status);
  const [sidebarWidth, setSidebarWidth] = useState(280);

  const handleResize = useCallback((delta: number) => {
    setSidebarWidth((w) => Math.max(200, Math.min(500, w + delta)));
  }, []);

  // Navigate to summary only when status TRANSITIONS to "ended"
  // (prevents redirect loop on back-button navigation)
  useEffect(() => {
    if (status === "ended" && prevStatusRef.current !== "ended") {
      navigate("/summary");
    }
    prevStatusRef.current = status;
  }, [status, navigate]);

  // Redirect if no discussion (only on initial mount)
  useEffect(() => {
    if (status === "idle") {
      navigate("/");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <>
      <TopBar title={t("arena.title")} />

      <div className="flex min-h-0 flex-1 flex-col">
        {/* Status bar */}
        <div className="relative z-10 flex items-center justify-between border-b border-border px-4 py-2">
          <TurnIndicator turn={currentTurn} status={status} determiningOrder={determiningOrder} webSearchCount={webSearchCount} />
          <DiscussionControls status={status} userTurnActive={userTurnActive} />
        </div>

        {/* Main content + emotion sidebar */}
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

            {/* Synthesis streaming */}
            {(status === "synthesizing" || synthesisStreaming) && (
              <div className="border-t border-border bg-card p-4">
                <p className="mb-1 text-xs font-medium text-primary">
                  {t("arena.synthesizing")}
                </p>
                <p className="whitespace-pre-wrap text-sm text-foreground">
                  {synthesisStreaming}
                  <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
                </p>
              </div>
            )}

            {/* User input area (when it's user's turn) */}
            {userTurnActive && (
              <div className="border-t border-border p-4">
                <UserInputArea timeoutSecs={userTimeout} />
              </div>
            )}
          </div>

          {/* Resize divider + Emotion sidebar */}
          <ResizeDivider onResize={handleResize} />
          <EmotionSidebar width={sidebarWidth} />
        </div>
      </div>
    </>
  );
}
