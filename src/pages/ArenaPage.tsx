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

      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Status bar */}
        <div className="flex items-center justify-between border-b border-border px-4 py-2">
          <div className="flex items-center gap-3">
            <TurnIndicator turn={currentTurn} status={status} />
            {/* Emotion dots for gladiateurs */}
            <div className="flex items-center gap-1.5">
              {gladiateurs.map((g) => {
                const emo = emotions.get(g.id);
                if (!emo) return null;
                return (
                  <div key={g.id} className="flex items-center gap-1" title={g.name}>
                    <EmotionIndicator emotions={emo} />
                    <span className="text-[10px] text-muted-foreground">
                      {g.name.slice(0, 8)}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
          <DiscussionControls status={status} userTurnActive={userTurnActive} />
        </div>

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
    </>
  );
}
