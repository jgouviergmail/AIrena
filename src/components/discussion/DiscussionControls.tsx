import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Hand,
  OctagonX,
  Pause,
  Play,
  Square,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useArenaStore } from "@/stores/useArenaStore";
import * as api from "@/lib/tauri-api";

export function DiscussionControls({
  status,
  userTurnActive,
}: {
  status: "running" | "paused" | "synthesizing" | "ended" | "idle";
  userTurnActive: boolean;
}) {
  const { t } = useTranslation();
  const dispatchEvent = useArenaStore((s) => s.handleEvent);
  const interventionRequested = useArenaStore((s) => s.interventionRequested);
  const [confirmHardStop, setConfirmHardStop] = useState(false);

  const handleError = (e: unknown) => {
    const msg = e instanceof Error ? e.message : String(e);
    dispatchEvent({ type: "error", data: { message: msg } });
  };

  const handlePauseResume = async () => {
    try {
      if (status === "paused") {
        await api.resumeDiscussion();
      } else {
        await api.pauseDiscussion();
      }
    } catch (e) {
      handleError(e);
    }
  };

  const handleSoftStop = async () => {
    try {
      await api.stopDiscussion();
    } catch (e) {
      handleError(e);
    }
  };

  const handleHardStop = async () => {
    if (!confirmHardStop) {
      setConfirmHardStop(true);
      setTimeout(() => setConfirmHardStop(false), 3000);
      return;
    }
    try {
      await api.forceStopDiscussion();
    } catch (e) {
      handleError(e);
    }
    setConfirmHardStop(false);
  };

  const handleIntervene = async () => {
    try {
      await api.userWantsToIntervene();
      useArenaStore.setState({ interventionRequested: true });
    } catch (e) {
      handleError(e);
    }
  };

  if (status === "ended" || status === "idle") return null;

  return (
    <div className="flex items-center gap-2">
      {status !== "synthesizing" && (
        <>
          <button
            onClick={handleIntervene}
            disabled={userTurnActive || interventionRequested || status === "paused"}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition-colors",
              interventionRequested
                ? "border-2 border-primary bg-primary/10 text-primary"
                : "border border-border text-foreground hover:bg-accent disabled:opacity-30",
            )}
          >
            <Hand className="h-3.5 w-3.5" />
            {interventionRequested ? t("arena.interveneRequested") : t("arena.intervene")}
          </button>

          <button
            onClick={handlePauseResume}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-foreground transition-colors hover:bg-accent"
          >
            {status === "paused" ? (
              <>
                <Play className="h-3.5 w-3.5" />
                {t("arena.resume")}
              </>
            ) : (
              <>
                <Pause className="h-3.5 w-3.5" />
                {t("arena.pause")}
              </>
            )}
          </button>

          <button
            onClick={handleSoftStop}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-foreground transition-colors hover:bg-accent"
          >
            <Square className="h-3.5 w-3.5" />
            {t("arena.softStop")}
          </button>
        </>
      )}

      <button
        onClick={handleHardStop}
        className={cn(
          "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition-colors",
          confirmHardStop
            ? "bg-destructive text-destructive-foreground"
            : "border border-destructive/30 text-destructive hover:bg-destructive/10",
        )}
      >
        <OctagonX className="h-3.5 w-3.5" />
        {confirmHardStop ? t("arena.hardStopConfirm") : t("arena.hardStop")}
      </button>
    </div>
  );
}
