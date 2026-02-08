import { useCallback, useEffect, useMemo } from "react";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useTokenBuffer } from "@/hooks/useTokenBuffer";
import { MessageBubble, StreamingBubble } from "./MessageBubble";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";

export function DiscussionFeed() {
  const messages = useArenaStore((s) => s.messages);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);

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
    <div className="flex-1 space-y-3 overflow-y-auto p-4">
      {messages.map((msg) => (
        <MessageBubble
          key={msg.id}
          message={msg}
          isActive={msg.speakerId === activeSpeakerId}
          emoji={emojiMap.get(msg.speakerId)}
          participantNames={participantNames}
          emojiMap={emojiMap}
        />
      ))}

      {/* Streaming messages (in-progress) */}
      {Array.from(streamingContent.entries()).map(([speakerId, content]) => {
        if (!content) return null;
        return (
          <StreamingBubble
            key={`stream-${speakerId}`}
            speakerName={resolveName(speakerId)}
            role={resolveRole(speakerId)}
            content={content}
            emoji={emojiMap.get(speakerId)}
            participantNames={participantNames}
          />
        );
      })}
    </div>
  );
}
