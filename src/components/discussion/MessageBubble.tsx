import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, Brain, Globe } from "lucide-react";
import { SpeakerBadge } from "./SpeakerBadge";
import { MathText } from "@/components/shared/MathText";
import { cn } from "@/lib/utils";
import type { Message, SpeakerRole } from "@/lib/types";

/** Stable empty array to avoid re-creating [] on every render (breaks useMemo deps). */
const EMPTY_NAMES: string[] = [];

/** Extract short forms from names for better matching.
 *  "Le Scientifique" → ["Scientifique"]
 *  "Napoléon Bonaparte" → ["Napoléon", "Bonaparte"]
 *  "L'Avocat du Diable" → ["Avocat du Diable"] */
function extractShortForms(name: string): string[] {
  const shorts: string[] = [];
  // Remove leading French/English articles: "Le ", "La ", "L'", "Les ", "The ", "A ", "An "
  const articleMatch = name.match(/^(?:Le |La |L'|Les |The |An? )/i);
  if (articleMatch) {
    const base = name.slice(articleMatch[0].length);
    if (base.length >= 3) {
      shorts.push(base);
    }
  }
  // Split multi-word names into individual parts (min 3 chars, skip articles/prepositions)
  const stopWords = new Set(["le", "la", "les", "l'", "du", "de", "des", "the", "of", "a", "an"]);
  const parts = name.split(/\s+/).filter(
    (p) => p.length >= 3 && !stopWords.has(p.toLowerCase()),
  );
  if (parts.length > 1) {
    for (const part of parts) {
      shorts.push(part);
    }
  }
  return shorts;
}

/** Split text by participant names, returning alternating text/highlighted segments */
function highlightNames(
  text: string,
  participantNames: string[],
): React.ReactNode[] {
  // Filter out empty/falsy names to avoid regex issues
  const validNames = participantNames.filter((n) => n && n.length > 1);
  if (validNames.length === 0) return [<MathText key={0} text={text} />];
  // Build expanded list: full names + short forms (article-stripped)
  const allForms = new Set<string>();
  for (const n of validNames) {
    allForms.add(n);
    for (const short of extractShortForms(n)) {
      allForms.add(short);
    }
  }
  // Sort longest-first so "Le Scientifique" matches before "Scientifique"
  const sorted = [...allForms].sort((a, b) => b.length - a.length);
  const escaped = sorted.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const regex = new RegExp(`(${escaped.join("|")})`, "gi");
  const parts = text.split(regex);
  const lowerNames = new Set(sorted.map((n) => n.toLowerCase()));
  return parts.map((part, i) => {
    if (lowerNames.has(part.toLowerCase())) {
      return (
        <span key={i} className="font-semibold text-primary">
          {part}
        </span>
      );
    }
    return <MathText key={i} text={part} />;
  });
}

export function MessageBubble({
  message,
  streaming,
  isActive,
  emoji,
  searchCount,
  wikiSearchCount,
  wikiArticleUrls,
  participantNames = EMPTY_NAMES,
  emojiMap,
}: {
  message: Message;
  streaming?: string;
  isActive?: boolean;
  emoji?: string;
  searchCount?: number;
  wikiSearchCount?: number;
  wikiArticleUrls?: string[];
  participantNames?: string[];
  emojiMap?: Map<string, string>;
}) {
  const { t } = useTranslation();
  const [showThought, setShowThought] = useState(false);
  const content = streaming ?? message.content;
  const isStreaming = streaming !== undefined;
  const highlighted = useMemo(
    () => highlightNames(content, participantNames),
    [content, participantNames],
  );

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
        <div className="flex items-center gap-2">
          <SpeakerBadge
            name={message.speakerName}
            role={message.role}
            active={isActive}
            emoji={emoji}
          />
          {(searchCount ?? 0) > 0 && (
            <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-1.5 py-0.5 text-[10px] font-medium text-blue-500" title={`${searchCount} web search(es)`}>
              <Globe className="h-3 w-3" />
            </span>
          )}
          {(wikiSearchCount ?? 0) > 0 && (
            <span
              className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-1.5 py-0.5 text-[10px] font-medium text-green-500 cursor-pointer"
              title={wikiArticleUrls?.join("\n") ?? `${wikiSearchCount} Wikipedia`}
              onClick={() => wikiArticleUrls?.[0] && window.open(wikiArticleUrls[0], "_blank")}
            >
              <BookOpen className="h-3 w-3" />
            </span>
          )}
        </div>
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
          {highlighted}
          {isStreaming && (
            <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
          )}
        </p>
      </div>

      {(message.reactions?.length ?? 0) > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
          {(message.reactions ?? []).map((r, i) => {
            const reactorEmoji = emojiMap?.get(r.fromSpeakerId) ?? "";
            return (
              <span
                key={i}
                className={cn(
                  "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px]",
                  r.reactionType === "like"
                    ? "bg-green-500/10 text-green-500"
                    : "bg-red-500/10 text-red-500",
                )}
                title={r.justification ?? undefined}
              >
                {r.reactionType === "like" ? "👍" : "👎"}
                {reactorEmoji && <span>{reactorEmoji}</span>}
                {r.fromSpeakerName}
              </span>
            );
          })}
        </div>
      )}
    </div>
  );
}

export function StreamingBubble({
  speakerName,
  role,
  content,
  emoji,
  participantNames = EMPTY_NAMES,
}: {
  speakerName: string;
  role: SpeakerRole;
  content: string;
  emoji?: string;
  participantNames?: string[];
}) {
  const highlighted = useMemo(
    () => highlightNames(content, participantNames),
    [content, participantNames],
  );
  return (
    <div className="rounded-lg border border-primary/30 bg-primary/5 p-4">
      <div className="mb-2">
        <SpeakerBadge name={speakerName} role={role} active emoji={emoji} />
      </div>
      <div className="text-sm text-foreground">
        <p className="whitespace-pre-wrap">
          {highlighted}
          <span className="inline-block h-4 w-1 animate-pulse bg-primary" />
        </p>
      </div>
    </div>
  );
}
