import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pause, Play, Square } from "lucide-react";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { StageView, type StageParticipant } from "@/components/stage/ArenaStage";
import { TimelineBar } from "@/components/stage/TimelineBar";
import { REPLAY_SPEEDS, replayDuration, scheduleReplay, type ReplaySpeed } from "@/lib/replay";
import type { DiscussionReport } from "@/lib/report";
import type { BanInfo, EmotionalProfile, Message, ParticipantInfo, ReactionType } from "@/lib/types";
import { cn } from "@/lib/utils";

/**
 * Replays a finished discussion at its real rhythm (timestamps), sped up by a
 * factor, with the stage animated from the persisted emotion history and the
 * reactions of each message. Older discussions without a report replay the
 * messages alone (stage without emotions).
 */
export function ReplayPlayer({ messages, participants, report }: { messages: Message[]; participants: ParticipantInfo[]; report: DiscussionReport }) {
  const { t } = useTranslation();
  const [speed, setSpeed] = useState<ReplaySpeed>(4);
  const [playing, setPlaying] = useState(false);
  const [revealed, setRevealed] = useState(0);
  const [bursts, setBursts] = useState<{ id: string; from: string; to: string; reactionType: ReactionType }[]>([]);
  const startedAt = useRef<number | null>(null);
  const elapsedBefore = useRef(0);

  const schedule = useMemo(() => scheduleReplay(messages, speed), [messages, speed]);
  const duration = replayDuration(schedule);

  // Clock: reveal the messages whose time has come
  useEffect(() => {
    if (!playing) return;
    startedAt.current = performance.now();
    const tick = () => {
      const elapsed = elapsedBefore.current + (performance.now() - (startedAt.current ?? performance.now()));
      const due = schedule.filter((s) => s.at <= elapsed).length;
      setRevealed((r) => (due > r ? due : r));
      if (due >= schedule.length) setPlaying(false);
    };
    const id = window.setInterval(tick, 100);
    return () => {
      window.clearInterval(id);
      elapsedBefore.current += performance.now() - (startedAt.current ?? performance.now());
    };
  }, [playing, schedule]);

  // A newly revealed message sends its reactions flying on the stage
  useEffect(() => {
    if (revealed === 0) return;
    const m = messages[revealed - 1];
    if (!m || m.reactions.length === 0) return;
    setBursts(m.reactions.slice(0, 6).map((r, i) => ({ id: `${m.id}-${i}`, from: r.fromSpeakerId, to: m.speakerId, reactionType: r.reactionType })));
  }, [revealed, messages]);

  const shown = messages.slice(0, revealed);
  const current = shown[shown.length - 1] ?? null;
  const currentTurn = current?.turnNumber ?? 0;
  const stageParticipants = useMemo<StageParticipant[]>(() => participants.filter((p) => p.role !== "user").map((p) => ({ id: p.id, name: p.name, emoji: p.emoji })), [participants]);
  const gladiateurCount = participants.filter((p) => p.role === "GladIAteur").length;
  const emotions = useMemo(() => {
    const map = new Map<string, EmotionalProfile>();
    for (const [id, history] of Object.entries(report.emotionHistory)) {
      const snapshot = [...history].reverse().find((h) => h.turn <= Math.max(1, currentTurn)) ?? history[0];
      if (snapshot) map.set(id, snapshot.emotions);
    }
    return map;
  }, [report.emotionHistory, currentTurn]);
  const bans = useMemo(() => {
    const map = new Map<string, BanInfo>();
    for (const e of report.timeline) {
      if (e.kind === "ban" && e.speakerId && e.turn <= currentTurn) map.set(e.speakerId, { remaining: 1, totalBans: 1, justIssued: false });
      if (e.kind === "banLifted" && e.speakerId && e.turn <= currentTurn) map.delete(e.speakerId);
    }
    return map;
  }, [report.timeline, currentTurn]);

  const restart = () => {
    setPlaying(false);
    setRevealed(0);
    setBursts([]);
    elapsedBefore.current = 0;
  };
  const progress = duration > 0 ? Math.min(100, Math.round((Math.min(elapsedBefore.current, duration) / duration) * 100)) : 0;

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-card px-3 py-2">
        <button
          type="button"
          onClick={() => setPlaying((p) => !p)}
          disabled={messages.length === 0 || revealed >= messages.length}
          className="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-40"
        >
          {playing ? <Pause className="h-3.5 w-3.5" /> : <Play className="h-3.5 w-3.5" />}
          {playing ? t("replay.pause") : t("replay.play")}
        </button>
        <button type="button" onClick={restart} className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-accent">
          <Square className="h-3.5 w-3.5" />
          {t("replay.restart")}
        </button>
        <div className="flex items-center gap-1" role="group" aria-label={t("replay.speed")}>
          {REPLAY_SPEEDS.map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => { restart(); setSpeed(s); }}
              className={cn("rounded-md border px-2 py-1 text-xs", s === speed ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-accent")}
            >
              ×{s}
            </button>
          ))}
        </div>
        <span className="ml-auto text-xs tabular-nums text-muted-foreground">{revealed} / {messages.length} · {progress} %</span>
      </div>
      <div className="overflow-hidden rounded-lg border border-border">
        <StageView
          participants={stageParticipants}
          gladiateurCount={gladiateurCount}
          activeSpeakerId={current?.speakerId ?? null}
          speakerOrder={[]}
          bans={bans}
          passed={[]}
          emotions={emotions}
          coalition={null}
          roomMood={null}
          bursts={bursts}
          onBurstDone={(id) => setBursts((b) => b.filter((x) => x.id !== id))}
        />
        <TimelineBar timeline={report.timeline} turns={messages.reduce((m, x) => Math.max(m, x.turnNumber), 0)} currentTurn={currentTurn} />
      </div>
      {shown.length > 0 ? (
        <ReadOnlyFeed messages={shown} participants={participants} sources={report.sources} />
      ) : (
        <p className="py-6 text-center text-sm italic text-muted-foreground">{t("replay.hint")}</p>
      )}
    </div>
  );
}
