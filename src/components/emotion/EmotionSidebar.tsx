import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BarChart3, Hexagon } from "lucide-react";
import { ParticipantEmotionCard, type EmotionView } from "./ParticipantEmotionCard";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { adjustEmotion } from "@/lib/tauri-api";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { cn } from "@/lib/utils";
import type { EmotionalProfile, EmotionSnapshot } from "@/lib/types";

const EMPTY_HISTORY: EmotionSnapshot[] = [];
const DEFAULT_EMOTIONS: EmotionalProfile = {
  engagement: 50,
  accord: 50,
  confiance: 50,
  frustration: 10,
  curiosite: 50,
  enthousiasme: 50,
};
/** How long a crossed threshold keeps its axis pulsing (ms). */
const THRESHOLD_FLASH_MS = 3000;
const VIEW_STORAGE_KEY = "airena.emotions.view";

function readView(): EmotionView {
  try {
    return localStorage.getItem(VIEW_STORAGE_KEY) === "radar" ? "radar" : "bars";
  } catch {
    return "bars";
  }
}

/** Emotion panel content (one card per participant, bars or radar view). */
export function EmotionPanel() {
  const { t } = useTranslation();
  const [view, setView] = useState<EmotionView>(readView);
  const [thresholdFlash, setThresholdFlash] = useState<Map<string, string>>(new Map());
  const thresholdTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const emotions = useArenaStore((s) => s.emotions);
  const emotionHistory = useArenaStore((s) => s.emotionHistory);
  const moodSummary = useArenaStore((s) => s.moodSummary);
  const directives = useArenaStore((s) => s.directives);
  const bans = useArenaStore((s) => s.bans);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const lastThresholdCrossed = useArenaStore((s) => s.lastThresholdCrossed);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const emotionDriven = useSettingsStore((s) => s.settings.emotionDriven);

  // Clean up all threshold timers on unmount
  useEffect(() => {
    const timers = thresholdTimers.current;
    return () => {
      timers.forEach((timer) => clearTimeout(timer));
      timers.clear();
    };
  }, []);

  // Handle threshold flash with tracked timers
  const handleThreshold = useCallback((speakerId: string, axis: string) => {
    const prev = thresholdTimers.current.get(speakerId);
    if (prev) clearTimeout(prev);

    setThresholdFlash((map) => new Map(map).set(speakerId, axis));

    const timer = setTimeout(() => {
      setThresholdFlash((map) => {
        const next = new Map(map);
        if (next.get(speakerId) === axis) next.delete(speakerId);
        return next;
      });
      thresholdTimers.current.delete(speakerId);
    }, THRESHOLD_FLASH_MS);
    thresholdTimers.current.set(speakerId, timer);
  }, []);

  // Flash the axis whenever the store records a new threshold crossing
  useEffect(() => {
    if (lastThresholdCrossed) {
      handleThreshold(lastThresholdCrossed.speakerId, lastThresholdCrossed.axis);
    }
  }, [lastThresholdCrossed, handleThreshold]);

  const handleAdjust = useCallback((speakerId: string, axis: string, value: number) => {
    // Optimistic update
    useArenaStore.setState((s) => {
      const em = new Map(s.emotions);
      const current = em.get(speakerId);
      if (current) em.set(speakerId, { ...current, [axis]: value });
      return { emotions: em };
    });
    // Fire-and-forget backend call
    adjustEmotion(speakerId, axis, value).catch(() => {});
  }, []);

  const switchView = (next: EmotionView) => {
    setView(next);
    try {
      localStorage.setItem(VIEW_STORAGE_KEY, next);
    } catch { /* ignore */ }
  };

  // Build participants list: arbitre first, then gladiateurs
  const participants = [
    { id: arbitre.id, name: arbitre.name, emoji: ROLE_EMOJIS.IArbitre, role: "IArbitre" },
    ...gladiateurs.map((g) => ({
      id: g.id,
      name: g.name,
      emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
      role: "GladIAteur",
    })),
  ];

  return (
    <>
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-border px-3 py-1.5">
        <span className="text-[10px] text-muted-foreground">
          {emotionDriven ? t("emotions.sidebar.drivenNote") : t("emotions.sidebar.disabledNote")}
        </span>
        <div className="flex shrink-0 gap-0.5 rounded-md border border-border p-0.5">
          {(["bars", "radar"] as const).map((v) => (
            <button
              key={v}
              onClick={() => switchView(v)}
              title={t(`emotions.view.${v}`)}
              aria-pressed={view === v}
              className={cn(
                "rounded p-1 transition-colors",
                view === v ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground",
              )}
            >
              {v === "bars" ? <BarChart3 className="h-3 w-3" /> : <Hexagon className="h-3 w-3" />}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 space-y-2 overflow-y-auto p-2">
        {participants.map((p) => (
          <ParticipantEmotionCard
            key={p.id}
            participant={p}
            emotions={emotions.get(p.id) ?? DEFAULT_EMOTIONS}
            history={emotionHistory.get(p.id) ?? EMPTY_HISTORY}
            onAdjust={(axis, value) => handleAdjust(p.id, axis, value)}
            thresholdAxis={thresholdFlash.get(p.id)}
            moodSummary={moodSummary.get(p.id)}
            currentTurn={currentTurn}
            directive={directives.get(p.id)}
            banInfo={bans.get(p.id)}
            view={view}
            isActive={p.id === activeSpeakerId}
          />
        ))}
      </div>
    </>
  );
}
