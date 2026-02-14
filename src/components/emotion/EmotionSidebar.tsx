import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { ParticipantEmotionCard } from "./ParticipantEmotionCard";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { adjustEmotion } from "@/lib/tauri-api";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
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

export function EmotionSidebar({ width = 280 }: { width?: number }) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [thresholdFlash, setThresholdFlash] = useState<Map<string, string>>(
    new Map(),
  );
  const thresholdTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(
    new Map(),
  );

  const emotions = useArenaStore((s) => s.emotions);
  const emotionHistory = useArenaStore((s) => s.emotionHistory);
  const moodSummary = useArenaStore((s) => s.moodSummary);
  const directives = useArenaStore((s) => s.directives);
  const bans = useArenaStore((s) => s.bans);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const emotionDriven = useSettingsStore((s) => s.settings.emotionDriven);

  // Clean up all threshold timers on unmount
  useEffect(() => {
    return () => {
      thresholdTimers.current.forEach((t) => clearTimeout(t));
      thresholdTimers.current.clear();
    };
  }, []);

  // Handle threshold flash with tracked timers
  const handleThreshold = useCallback(
    (speakerId: string, axis: string) => {
      // Clear previous timer for this speaker
      const prev = thresholdTimers.current.get(speakerId);
      if (prev) clearTimeout(prev);

      setThresholdFlash((map) => {
        const next = new Map(map);
        next.set(speakerId, axis);
        return next;
      });

      // Auto-clear after 3s
      const timer = setTimeout(() => {
        setThresholdFlash((map) => {
          const next = new Map(map);
          if (next.get(speakerId) === axis) {
            next.delete(speakerId);
          }
          return next;
        });
        thresholdTimers.current.delete(speakerId);
      }, 3000);
      thresholdTimers.current.set(speakerId, timer);
    },
    [],
  );

  // Monkey-patch threshold handling into the existing event handler
  useEffect(() => {
    const originalHandler = useArenaStore.getState().handleEvent;
    const wrappedHandler = (event: Parameters<typeof originalHandler>[0]) => {
      if (
        event.type === "emotionalThresholdCrossed" &&
        "data" in event &&
        event.data
      ) {
        const data = event.data as {
          speakerId: string;
          axis: string;
        };
        handleThreshold(data.speakerId, data.axis);
      }
      originalHandler(event);
    };
    useArenaStore.setState({ handleEvent: wrappedHandler });
    return () => {
      useArenaStore.setState({ handleEvent: originalHandler });
    };
  }, [handleThreshold]);

  const handleAdjust = useCallback(
    (speakerId: string, axis: string, value: number) => {
      // Optimistic update
      useArenaStore.setState((s) => {
        const em = new Map(s.emotions);
        const current = em.get(speakerId);
        if (current) {
          em.set(speakerId, { ...current, [axis]: value });
        }
        return { emotions: em };
      });
      // Fire-and-forget backend call
      adjustEmotion(speakerId, axis, value).catch(() => {});
    },
    [],
  );

  // Build participants list: arbitre first, then gladiateurs
  const participants = [
    {
      id: arbitre.id,
      name: arbitre.name,
      emoji: ROLE_EMOJIS.IArbitre,
      role: "IArbitre",
    },
    ...gladiateurs.map((g) => ({
      id: g.id,
      name: g.name,
      emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
      role: "GladIAteur",
    })),
  ];

  if (isCollapsed) {
    return (
      <div className="flex w-8 shrink-0 flex-col items-center border-l border-border bg-card/50 pt-2">
        <button
          onClick={() => setIsCollapsed(false)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("emotions.sidebar.expand")}
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="mt-2 text-xs [writing-mode:vertical-lr] text-muted-foreground">
          {t("emotions.sidebar.title")}
        </span>
      </div>
    );
  }

  return (
    <div className="flex shrink-0 flex-col border-l border-border bg-card/50" style={{ width: `${width}px` }}>
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <span className="text-xs font-medium text-foreground">
          {t("emotions.sidebar.title")}
        </span>
        <button
          onClick={() => setIsCollapsed(true)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("emotions.sidebar.collapse")}
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>

      {!emotionDriven && (
        <div className="border-b border-border px-3 py-1.5 text-[10px] text-muted-foreground">
          {t("emotions.sidebar.disabledNote")}
        </div>
      )}

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
          />
        ))}
      </div>
    </div>
  );
}
