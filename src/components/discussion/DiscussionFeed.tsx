import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown } from "lucide-react";
import { parseStreamKey, useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useTokenBuffer } from "@/hooks/useTokenBuffer";
import { MessageBubble, StreamingBubble } from "./MessageBubble";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";

/** Distance from the bottom (px) under which the reader is considered "at the latest message". */
const NEAR_BOTTOM_PX = 120;

/** Separator between turns — anchors each turn in the feed. */
export function TurnDivider({ turn }: { turn: number }) {
  const { t } = useTranslation();
  return (
    <div id={`turn-${turn}`} className="flex items-center gap-3 py-1" role="separator" aria-label={t("arena.turn", { number: turn })}>
      <span className="h-px flex-1 bg-border" />
      <span className="rounded-full border border-border bg-card px-2.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        {t("arena.turn", { number: turn })}
      </span>
      <span className="h-px flex-1 bg-border" />
    </div>
  );
}

export function DiscussionFeed() {
  const { t } = useTranslation();
  const messages = useArenaStore((s) => s.messages);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const webSearchesPerMessage = useArenaStore((s) => s.webSearchesPerMessage);
  const wikiSearchesPerMessage = useArenaStore((s) => s.wikiSearchesPerMessage);
  const wikiArticleUrlsPerMessage = useArenaStore((s) => s.wikiArticleUrlsPerMessage);
  const ragChunksPerMessage = useArenaStore((s) => s.ragChunksPerMessage);
  const ragChunkDetailsPerMessage = useArenaStore((s) => s.ragChunkDetailsPerMessage);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [showJump, setShowJump] = useState(false);
  const atBottomRef = useRef(true);

  // Token buffer: collects streaming tokens and flushes every 60ms
  // instead of re-rendering on every single token
  const {
    flushed: streamingContent,
    pushToken,
    clearSpeaker,
    clearAll,
  } = useTokenBuffer(60);

  // Subscribe to raw streaming events from the store
  const registerBuffer = useArenaStore((s) => s.registerStreamBuffer);
  const unregisterBuffer = useArenaStore((s) => s.unregisterStreamBuffer);

  useEffect(() => {
    registerBuffer({ pushToken, clearSpeaker, clearAll });
    return () => unregisterBuffer();
  }, [registerBuffer, unregisterBuffer, pushToken, clearSpeaker, clearAll]);

  // No auto-scroll (it prevented reading) — instead, offer a "jump to latest"
  // button as soon as new content arrives while the reader is scrolled up.
  const handleScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_BOTTOM_PX;
    if (atBottomRef.current) setShowJump(false);
  }, []);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    // Content grows below the fold without firing a scroll event — measure it
    const away = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (away > NEAR_BOTTOM_PX) {
      atBottomRef.current = false;
      setShowJump(true);
    }
  }, [messages.length, streamingContent]);

  const jumpToLatest = () => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
    atBottomRef.current = true;
    setShowJump(false);
  };

  const resolveName = useCallback(
    (speakerId: string): string =>
      messages.find((m) => m.speakerId === speakerId)?.speakerName ??
      gladiateurs.find((g) => g.id === speakerId)?.name ??
      (arbitre.id === speakerId ? arbitre.name : speakerId),
    [messages, gladiateurs, arbitre],
  );

  const resolveRole = useCallback(
    (speakerId: string): "IArbitre" | "GladIAteur" =>
      arbitre.id === speakerId ? "IArbitre" : "GladIAteur",
    [arbitre],
  );

  // Pre-compute emoji map from gladiateur/arbitre configs
  const emojiMap = useMemo(() => {
    const map = new Map<string, string>();
    map.set(arbitre.id, ROLE_EMOJIS.IArbitre);
    for (const g of gladiateurs) {
      map.set(g.id, g.emoji ?? getProfileEmoji(g.name, g.systemPrompt));
    }
    map.set("user", ROLE_EMOJIS.user);
    return map;
  }, [gladiateurs, arbitre]);

  // All participant names for highlighting mentions in messages
  const participantNames = useMemo(
    () => [arbitre.name, ...gladiateurs.map((g) => g.name)],
    [arbitre.name, gladiateurs],
  );

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div ref={scrollRef} onScroll={handleScroll} className="flex-1 space-y-3 overflow-y-auto p-3 sm:p-4">
        {messages.map((msg, i) => {
          const newTurn = msg.turnNumber > 0 && (i === 0 || messages[i - 1].turnNumber !== msg.turnNumber);
          return (
            <Fragment key={msg.id}>
              {newTurn && <TurnDivider turn={msg.turnNumber} />}
              <MessageBubble
                message={msg}
                isActive={msg.speakerId === activeSpeakerId}
                emoji={emojiMap.get(msg.speakerId)}
                searchCount={webSearchesPerMessage[msg.id]}
                wikiSearchCount={wikiSearchesPerMessage[msg.id]}
                wikiArticleUrls={wikiArticleUrlsPerMessage[msg.id]}
                ragChunkCount={ragChunksPerMessage[msg.id]}
                ragChunkDetails={ragChunkDetailsPerMessage[msg.id]}
                participantNames={participantNames}
                emojiMap={emojiMap}
              />
            </Fragment>
          );
        })}

        {/* Streaming messages (in-progress): live reasoning first, then content */}
        {Array.from(streamingContent.entries())
          .map(([key, content]) => ({ key, content, ...parseStreamKey(key) }))
          .filter((s) => s.content)
          .sort((a, b) => (a.kind === b.kind ? 0 : a.kind === "reasoning" ? -1 : 1))
          .map((s) => (
            <StreamingBubble
              key={`stream-${s.key}`}
              speakerName={resolveName(s.speakerId)}
              role={resolveRole(s.speakerId)}
              content={s.content}
              emoji={emojiMap.get(s.speakerId)}
              participantNames={participantNames}
              variant={s.kind}
            />
          ))}
      </div>

      {showJump && (
        <button
          onClick={jumpToLatest}
          className="absolute bottom-3 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1.5 rounded-full border border-border bg-card/95 px-3 py-1.5 text-xs font-medium text-foreground shadow-md backdrop-blur hover:bg-accent motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2"
        >
          <ArrowDown className="h-3.5 w-3.5" />
          {t("arena.jumpToLatest")}
        </button>
      )}
    </div>
  );
}
