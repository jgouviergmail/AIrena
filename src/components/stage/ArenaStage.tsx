import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { arcPositions, BURST_MS, dominantEmotion, EMOTION_HUE, USER_SPEAKER_ID, userHasSpoken, type StagePoint } from "@/lib/stage";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { REACTION_EMOJI } from "@/lib/reactions";
import { cn } from "@/lib/utils";
import type { BanInfo, EmotionalProfile, RoomMood } from "@/lib/types";
import type { ReactionBurst } from "@/stores/arena/types";

const STAGE_HEIGHT = 168;
const STAGE_HEIGHT_NARROW = 132;
const AVATAR = 44;
/** Aura radius grows with the intensity of the dominant emotion (px). */
const AURA_MIN = 6;
const AURA_MAX = 18;

const MOOD_TINT: Record<RoomMood, string> = {
  tense: "oklch(0.55 0.18 25 / 0.14)",
  flat: "oklch(0.6 0.02 250 / 0.10)",
  lively: "oklch(0.7 0.16 150 / 0.14)",
  serene: "oklch(0.7 0.1 230 / 0.12)",
};

export interface StageParticipant {
  id: string;
  name: string;
  emoji: string;
  /** Role or hat label of the turn (structured modes, v1.19) */
  role?: string;
}

/** One reaction flying from its author to the target, then gone. */
function Burst({ id, emoji, from, to, onDone }: { id: string; emoji: string; from: StagePoint; to: StagePoint; onDone?: (id: string) => void }) {
  useEffect(() => {
    const timer = window.setTimeout(() => onDone?.(id), BURST_MS);
    return () => window.clearTimeout(timer);
  }, [id, onDone]);
  return (
    <span
      aria-hidden="true"
      className="pointer-events-none absolute text-lg motion-safe:animate-[arena-burst_900ms_ease-out_forwards] motion-reduce:hidden"
      style={{ left: from.x, top: from.y, ["--burst-dx" as string]: `${to.x - from.x}px`, ["--burst-dy" as string]: `${to.y - from.y}px` } as React.CSSProperties}
    >
      {emoji}
    </span>
  );
}

/** Everything the stage draws (the live arena feeds it from the store, the replay from the report). */
export interface StageProps {
  participants: StageParticipant[];
  gladiateurCount: number;
  activeSpeakerId: string | null;
  speakerOrder: string[];
  bans: Map<string, BanInfo>;
  passed: string[];
  emotions: Map<string, EmotionalProfile>;
  coalition: { a: string; b: string } | null;
  roomMood: RoomMood | null;
  bursts: ReactionBurst[];
  onBurstDone?: (id: string) => void;
}

/** The stage of the live arena, fed by the arena store. */
export function ArenaStage() {
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const speakerOrder = useArenaStore((s) => s.speakerOrder);
  const bans = useArenaStore((s) => s.bans);
  const passed = useArenaStore((s) => s.passedSpeakerIds);
  const emotions = useArenaStore((s) => s.emotions);
  const coalition = useArenaStore((s) => s.coalition);
  const roomMood = useArenaStore((s) => s.roomMood);
  const bursts = useArenaStore((s) => s.bursts);
  const dismissBurst = useArenaStore((s) => s.dismissBurst);
  const roles = useArenaStore((s) => s.roles);
  // The audience member takes a seat on the stage once they have spoken (v1.20.2)
  const userSpoke = useArenaStore((s) => userHasSpoken(s.messages));
  const userTurnActive = useArenaStore((s) => s.userTurnActive);
  const username = useSettingsStore((s) => s.settings.username);
  const { t } = useTranslation();
  const participants = useMemo<StageParticipant[]>(
    () => [
      { id: arbitre.id, name: arbitre.name, emoji: ROLE_EMOJIS.IArbitre },
      ...gladiateurs.map((g) => {
        const assignment = roles.get(g.id);
        return { id: g.id, name: g.name, emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt), role: assignment ? t(`roles.${assignment.role}`, assignment.label) : undefined };
      }),
      ...(userSpoke || userTurnActive ? [{ id: USER_SPEAKER_ID, name: username, emoji: ROLE_EMOJIS.user, role: t("stage.audienceSeat") }] : []),
    ],
    [arbitre, gladiateurs, roles, t, userSpoke, userTurnActive, username],
  );
  return (
    <StageView
      participants={participants}
      gladiateurCount={gladiateurs.length}
      activeSpeakerId={userTurnActive ? USER_SPEAKER_ID : activeSpeakerId}
      speakerOrder={speakerOrder}
      bans={bans}
      passed={passed}
      emotions={emotions}
      coalition={coalition}
      roomMood={roomMood?.label ?? null}
      bursts={bursts}
      onBurstDone={dismissBurst}
    />
  );
}

/**
 * The arena seen from the audience: the participants on an arc, the active
 * speaker under the spotlight, the others dimmed, the banned behind bars, the
 * ones who passed greyed, an aura coloured by the dominant emotion, a link
 * between the allies of a coalition and the reactions flying across. Scrolls
 * horizontally on narrow screens.
 */
export function StageView({ participants, gladiateurCount, activeSpeakerId, speakerOrder, bans, passed, emotions, coalition, roomMood, bursts, onBurstDone }: StageProps) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(640);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => setWidth(Math.max(320, entry.contentRect.width)));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const narrow = width < 560;
  const height = narrow ? STAGE_HEIGHT_NARROW : STAGE_HEIGHT;
  const minWidth = Math.max(width, participants.length * 88);
  const positions = useMemo(() => arcPositions(participants.length, minWidth, height), [participants.length, minWidth, height]);
  const at = useMemo(() => new Map(participants.map((p, i) => [p.id, positions[i]])), [participants, positions]);
  const orderIndex = new Map(speakerOrder.map((id, i) => [id, i + 1]));

  return (
    <div
      ref={ref}
      role="img"
      aria-label={t("stage.label", { count: gladiateurCount })}
      className="relative shrink-0 overflow-x-auto overflow-y-hidden border-b border-border"
      style={{ height, background: `linear-gradient(180deg, var(--arena-stage-top), var(--arena-stage-bottom)), ${roomMood ? MOOD_TINT[roomMood] : "transparent"}` }}
    >
      <div className="relative" style={{ width: minWidth, height }}>
        {/* Spotlight on the active speaker */}
        {activeSpeakerId && at.get(activeSpeakerId) && (
          <div
            aria-hidden="true"
            className="pointer-events-none absolute -translate-x-1/2 -translate-y-1/2 rounded-full motion-safe:transition-all motion-safe:duration-500"
            style={{ left: at.get(activeSpeakerId)!.x, top: at.get(activeSpeakerId)!.y, width: AVATAR * 3, height: AVATAR * 3, background: "var(--arena-spotlight)" }}
          />
        )}
        {/* Coalition link */}
        {coalition && at.get(coalition.a) && at.get(coalition.b) && (
          <svg aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full">
            <line
              x1={at.get(coalition.a)!.x} y1={at.get(coalition.a)!.y} x2={at.get(coalition.b)!.x} y2={at.get(coalition.b)!.y}
              className="stroke-green-500/70" strokeWidth={2} strokeDasharray="4 3"
            />
          </svg>
        )}
        {participants.map((p) => {
          const pos = at.get(p.id)!;
          const ban = bans.get(p.id);
          const banned = (ban?.remaining ?? 0) > 0;
          const hasPassed = passed.includes(p.id);
          const active = p.id === activeSpeakerId;
          const dominant = emotions.has(p.id) ? dominantEmotion(emotions.get(p.id)!) : null;
          const auraSize = dominant ? AURA_MIN + ((AURA_MAX - AURA_MIN) * Math.min(50, dominant.intensity)) / 50 : 0;
          const aura = dominant ? `0 0 ${auraSize * 2}px ${auraSize}px oklch(0.7 0.18 ${EMOTION_HUE[dominant.axis]} / 0.55)` : undefined;
          const rank = orderIndex.get(p.id);
          return (
            <div
              key={p.id}
              className={cn(
                "absolute flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1 motion-safe:transition-all motion-safe:duration-500",
                !active && "opacity-70",
                hasPassed && "opacity-40 grayscale",
                banned && "opacity-50",
              )}
              style={{ left: pos.x, top: pos.y }}
              title={p.name}
            >
              <div
                className={cn(
                  "relative flex items-center justify-center rounded-full bg-card text-2xl ring-1 ring-border",
                  active && "ring-2 ring-primary motion-safe:animate-pulse",
                )}
                style={{ width: AVATAR, height: AVATAR, boxShadow: aura }}
              >
                <span aria-hidden="true">{p.emoji}</span>
                {banned && (
                  <span className="absolute inset-0 flex items-center justify-center rounded-full bg-background/70 text-base" title={t("stage.banned")}>🚫</span>
                )}
                {rank && !banned && (
                  <span className="absolute -right-1 -top-1 rounded-full bg-primary px-1 text-[9px] font-semibold text-primary-foreground">{rank}</span>
                )}
              </div>
              <span className={cn("max-w-[84px] truncate text-[10px]", active ? "font-semibold text-foreground" : "text-muted-foreground")}>
                {p.name}
              </span>
              {p.role && (
                <span className="max-w-[84px] truncate rounded-full bg-primary/10 px-1.5 text-[9px] font-medium text-primary" title={p.role}>{p.role}</span>
              )}
            </div>
          );
        })}
        {bursts.map((b) => {
          const from = at.get(b.from);
          const to = at.get(b.to);
          return from && to ? <Burst key={b.id} id={b.id} emoji={REACTION_EMOJI[b.reactionType]} from={from} to={to} onDone={onBurstDone} /> : null;
        })}
      </div>
    </div>
  );
}
