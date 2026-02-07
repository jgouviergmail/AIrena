import { useEffect, useRef } from "react";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { MessageBubble, StreamingBubble } from "./MessageBubble";

export function DiscussionFeed() {
  const messages = useArenaStore((s) => s.messages);
  const streamingContent = useArenaStore((s) => s.streamingContent);
  const activeSpeakerId = useArenaStore((s) => s.activeSpeakerId);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Auto-scroll
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingContent]);

  return (
    <div className="flex-1 space-y-3 overflow-y-auto p-4">
      {messages.map((msg) => (
        <MessageBubble
          key={msg.id}
          message={msg}
          isActive={msg.speakerId === activeSpeakerId}
        />
      ))}

      {/* Streaming messages (in-progress) */}
      {Array.from(streamingContent.entries()).map(([speakerId, content]) => {
        if (!content) return null;
        // Find name from messages, config, or fall back to ID
        const name =
          messages.find((m) => m.speakerId === speakerId)?.speakerName ??
          gladiateurs.find((g) => g.id === speakerId)?.name ??
          (arbitre.id === speakerId ? arbitre.name : speakerId);
        return (
          <StreamingBubble
            key={`stream-${speakerId}`}
            speakerName={name}
            content={content}
          />
        );
      })}

      <div ref={bottomRef} />
    </div>
  );
}
