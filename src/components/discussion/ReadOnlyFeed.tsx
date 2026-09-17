import { useMemo } from "react";
import { MessageBubble } from "./MessageBubble";
import type { SourceRecord } from "@/lib/report";
import type { Message, ParticipantInfo } from "@/lib/types";

export function ReadOnlyFeed({
  messages,
  participants,
  sources = [],
}: {
  messages: Message[];
  participants: ParticipantInfo[];
  /** Persisted sources (report) to show under the messages they served */
  sources?: SourceRecord[];
}) {
  const sourcesPerMessage = useMemo(() => {
    const map = new Map<string, SourceRecord[]>();
    for (const s of sources) {
      if (!s.messageId) continue;
      map.set(s.messageId, [...(map.get(s.messageId) ?? []), s]);
    }
    return map;
  }, [sources]);
  const emojiMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const p of participants) {
      map.set(p.id, p.emoji);
    }
    return map;
  }, [participants]);

  const participantNames = useMemo(
    () => participants.map((p) => p.name),
    [participants],
  );

  return (
    <div className="space-y-3">
      {messages.map((msg) => (
        <MessageBubble
          key={msg.id}
          message={msg}
          emoji={emojiMap.get(msg.speakerId)}
          participantNames={participantNames}
          emojiMap={emojiMap}
          sources={sourcesPerMessage.get(msg.id)}
        />
      ))}
    </div>
  );
}
