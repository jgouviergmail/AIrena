import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { nextSpeaker, pauseDiscussion, resumeDiscussion, userWantsToIntervene } from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { neighbourTurn, shortcutAction } from "@/lib/stage";
import { scrollToTurn } from "@/components/stage/TimelineBar";
import { useArenaStore } from "@/stores/useArenaStore";
import { useUiStore } from "@/stores/useUiStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { toast } from "@/stores/useToastStore";
import { setPresentation } from "@/lib/presentation";

/**
 * Keyboard shortcuts of the arena (desktop): Space pause/resume, I intervene,
 * ← → turns, M mute, P projection, S sounds, ? help, Escape leaves the
 * projection. Nothing fires while typing in a field.
 */
export function useArenaShortcuts(enabled: boolean) {
  const { t } = useTranslation();

  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const action = shortcutAction({
        key: e.key,
        targetTag: target?.tagName,
        editable: target?.isContentEditable,
        ctrlKey: e.ctrlKey,
        metaKey: e.metaKey,
        altKey: e.altKey,
      });
      if (!action) return;
      e.preventDefault();
      const arena = useArenaStore.getState();
      const ui = useUiStore.getState();
      const run = (p: Promise<void>) => p.catch((err) => toast.error(t("errors.generic"), extractErrorMessage(err)));
      switch (action) {
        case "pauseResume":
          if (arena.status === "paused") run(resumeDiscussion());
          else if (arena.status === "running") run(pauseDiscussion());
          break;
        case "intervene":
          if (arena.status === "running" && !arena.userTurnActive && !arena.interventionRequested) {
            run(userWantsToIntervene().then(() => useArenaStore.setState({ interventionRequested: true })));
          }
          break;
        case "previousTurn":
        case "nextTurn": {
          const turn = neighbourTurn(ui.viewedTurn ?? arena.currentTurn, action, arena.currentTurn);
          if (turn) {
            useUiStore.setState({ viewedTurn: turn });
            scrollToTurn(turn);
          }
          break;
        }
        case "mute": {
          const settings = useSettingsStore.getState();
          settings.updateSettings({ ttsEnabled: !settings.settings.ttsEnabled });
          break;
        }
        case "sounds": {
          const settings = useSettingsStore.getState();
          settings.updateSettings({ soundEnabled: !settings.settings.soundEnabled });
          break;
        }
        case "presentation":
          setPresentation(!ui.presentationMode);
          break;
        case "exitPresentation":
          if (ui.presentationMode) setPresentation(false);
          break;
        case "nextSpeaker":
          // Step mode: only while the engine waits for the cue
          if (arena.awaitingCue) run(nextSpeaker().then(() => useArenaStore.setState({ awaitingCue: null })));
          break;
        case "help":
          toast.info(t("shortcuts.title"), t("shortcuts.list"));
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled, t]);
}
