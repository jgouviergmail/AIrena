import { useMemo } from "react";
import { MessageBubble } from "./MessageBubble";
import type { Message, ParticipantInfo } from "@/lib/types";

export function ReadOnlyFeed({
  messages,
  participants,
}: {
  messages: Message[];
  participants: ParticipantInfo[];
}) {
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
        />
      ))}
    </div>
  );
}
