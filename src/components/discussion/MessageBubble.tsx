import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Brain, ThumbsDown, ThumbsUp } from "lucide-react";
import { SpeakerBadge } from "./SpeakerBadge";
import { cn } from "@/lib/utils";
import type { Message } from "@/lib/types";

export function MessageBubble({
  message,
  streaming,
  isActive,
}: {
  message: Message;
  streaming?: string;
  isActive?: boolean;
}) {
  const { t } = useTranslation();
  const [showThought, setShowThought] = useState(false);
  const content = streaming ?? message.content;
  const isStreaming = streaming !== undefined;

  if (message.isBanNotification) {
    return (
      <div className="flex justify-center py-2">
        <div className="rounded-full bg-destructive/10 px-4 py-1.5 text-xs text-destructive">
          {message.content}
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "group rounded-lg border border-border bg-card p-4 transition-colors",
        isActive && "border-primary/30 bg-primary/5",
      )}
    >
      <div className="mb-2 flex items-center justify-between">
        <SpeakerBadge
          name={message.speakerName}
          role={message.role}
          active={isActive}
        />
        <div className="flex items-center gap-2">
          {message.innerThought && (
            <button
              onClick={() => setShowThought(!showThought)}
              className="flex items-center gap-1 rounded px-1.5 py-0.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <Brain className="h-3 w-3" />
              {showThought ? t("arena.hideThoughts") : t("arena.showThoughts")}
            </button>
          )}
        </div>
      </div>

      {showThought && message.innerThought && (
        <div className="mb-3 rounded-md border border-dashed border-border bg-muted/30 p-3">
          <p className="text-xs italic text-muted-foreground">
            {message.innerThought}
          </p>
        </div>
      )}

      <div className="prose prose-sm max-w-none text-sm text-foreground">
        <p className="whitespace-pre-wrap">
          {content}
          {isStreaming && (
            <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
          )}
        </p>
      </div>

      {(message.reactions?.length ?? 0) > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
          {(message.reactions ?? []).map((r, i) => (
            <span
              key={i}
              className={cn(
                "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px]",
                r.reactionType === "like"
                  ? "bg-green-500/10 text-green-500"
                  : "bg-red-500/10 text-red-500",
              )}
            >
              {r.reactionType === "like" ? (
                <ThumbsUp className="h-2.5 w-2.5" />
              ) : (
                <ThumbsDown className="h-2.5 w-2.5" />
              )}
              {r.fromSpeakerName}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function StreamingBubble({
  speakerName,
  content,
}: {
  speakerName: string;
  content: string;
}) {
  return (
    <div className="rounded-lg border border-primary/30 bg-primary/5 p-4">
      <div className="mb-2">
        <SpeakerBadge name={speakerName} role="GladIAteur" active />
      </div>
      <div className="text-sm text-foreground">
        <p className="whitespace-pre-wrap">
          {content}
          <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
        </p>
      </div>
    </div>
  );
}
