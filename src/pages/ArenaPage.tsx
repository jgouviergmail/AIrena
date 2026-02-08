import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { TopBar } from "@/components/layout/TopBar";
import { DiscussionFeed } from "@/components/discussion/DiscussionFeed";
import { DiscussionControls } from "@/components/discussion/DiscussionControls";
import { UserInputArea } from "@/components/discussion/UserInputArea";
import { TurnIndicator } from "@/components/discussion/TurnIndicator";
import { EmotionIndicator } from "@/components/discussion/EmotionIndicator";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";

export default function ArenaPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const status = useArenaStore((s) => s.status);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const userTurnActive = useArenaStore((s) => s.userTurnActive);
  const emotions = useArenaStore((s) => s.emotions);
  const determiningOrder = useArenaStore((s) => s.determiningOrder);
  const error = useArenaStore((s) => s.error);
  const synthesisStreaming = useArenaStore((s) => s.synthesisStreaming);
  const userTimeout = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const prevStatusRef = useRef(status);

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
        {/* Status bar — outside overflow-hidden so tooltips are visible */}
        <div className="relative z-10 flex items-center justify-between border-b border-border px-4 py-2">
          <div className="flex items-center gap-3">
            <TurnIndicator turn={currentTurn} status={status} determiningOrder={determiningOrder} />
            {/* Emotion dots for gladiateurs */}
            <div className="flex items-center gap-1.5">
              {gladiateurs.map((g) => {
                const emo = emotions.get(g.id);
                if (!emo) return null;
                return (
                  <div key={g.id} className="group relative flex items-center gap-1">
                    <EmotionIndicator emotions={emo} />
                    <span className="text-[10px] text-muted-foreground">
                      {g.name.slice(0, 8)}
                    </span>
                    <div className="pointer-events-none absolute left-1/2 top-full z-50 mt-1 -translate-x-1/2 whitespace-nowrap rounded bg-popover px-2 py-1 text-[10px] text-popover-foreground shadow-md opacity-0 transition-opacity group-hover:opacity-100">
                      {g.name}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
          <DiscussionControls status={status} userTurnActive={userTurnActive} />
        </div>

        {/* Scrollable content area — overflow-hidden here, not on parent */}
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
      </div>
    </>
  );
}
