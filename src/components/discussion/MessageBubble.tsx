import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, Brain, Database, Globe, Link2 } from "lucide-react";
import { SourceRow } from "@/components/sources/SourcesList";
import { ReactionBar } from "./ReactionBar";
import type { SourceRecord } from "@/lib/report";
import { SpeakerBadge } from "./SpeakerBadge";
import { MathText } from "@/components/shared/MathText";
import { cn } from "@/lib/utils";
import { REACTION_EMOJI, REACTION_TONE_CLASS, reactionTone } from "@/lib/reactions";
import { openExternalUrl } from "@/lib/tauri-api";
import { messageKind, type IntentionData, type Message, type RagChunkInfo, type ReactionType, type SpeakerRole } from "@/lib/types";

/** Stable empty array to avoid re-creating [] on every render (breaks useMemo deps). */
const EMPTY_NAMES: string[] = [];

/** Normalize Unicode dash variants (en-dash, em-dash, non-breaking hyphen, etc.) to ASCII hyphen.
 *  LLMs often output U+2011 (non-breaking hyphen) instead of U+002D (ASCII hyphen). */
const DASH_RE = /[\u2010\u2011\u2012\u2013\u2014\u2015\u2212]/g;
function normalizeDashes(s: string): string {
  return s.replace(DASH_RE, "-");
}

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
  // Normalize dashes so "Le Psycho‑rigide" (U+2011) matches "Le Psycho-rigide" (U+002D)
  const allForms = new Set<string>();
  for (const n of validNames) {
    allForms.add(normalizeDashes(n));
    for (const short of extractShortForms(normalizeDashes(n))) {
      allForms.add(short);
    }
  }
  // Sort longest-first so "Le Scientifique" matches before "Scientifique"
  const sorted = [...allForms].sort((a, b) => b.length - a.length);
  // Escape regex special chars, then replace literal hyphens with a dash character class
  // so the pattern matches both ASCII hyphen and Unicode dash variants
  const escaped = sorted.map((n) =>
    n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
     .replace(/-/g, "[-\u2010\u2011\u2012\u2013\u2014\u2015\u2212]"),
  );
  // Use \b word boundaries to prevent matching "bar" inside "barrières"
  const regex = new RegExp(`\\b(${escaped.join("|")})\\b`, "gi");
  // Normalize dashes in input text so that lookup in lowerNames works
  const normalizedText = normalizeDashes(text);
  const parts = normalizedText.split(regex);
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
  ragChunkCount,
  ragChunkDetails,
  participantNames = EMPTY_NAMES,
  emojiMap,
  sources,
  audience,
}: {
  message: Message;
  streaming?: string;
  isActive?: boolean;
  emoji?: string;
  searchCount?: number;
  wikiSearchCount?: number;
  wikiArticleUrls?: string[];
  ragChunkCount?: number;
  ragChunkDetails?: RagChunkInfo[];
  participantNames?: string[];
  emojiMap?: Map<string, string>;
  /** References injected into this speaker's prompt for this message */
  sources?: SourceRecord[];
  /** Audience reactions (arena, running discussion): handler + cap */
  audience?: { onReact: (type: ReactionType) => void; max: number };
}) {
  const { t } = useTranslation();
  const [showThought, setShowThought] = useState(false);
  const [showSources, setShowSources] = useState(false);
  const content = streaming ?? message.content;
  const isStreaming = streaming !== undefined;
  const highlighted = useMemo(
    () => highlightNames(content, participantNames),
    [content, participantNames],
  );

  const kind = messageKind(message);
  if (kind === "banNotification") {
    return (
      <div className="flex justify-center py-2">
        <div className="rounded-full bg-destructive/10 px-4 py-1.5 text-xs text-destructive">
          {message.content}
        </div>
      </div>
    );
  }
  if (kind === "stageDirection") {
    return (
      <p className="px-6 py-1 text-center text-xs italic text-muted-foreground motion-safe:animate-in motion-safe:fade-in">
        {emoji && <span className="mr-1 not-italic">{emoji}</span>}
        {message.content}
      </p>
    );
  }
  if (kind === "actAnnouncement" || kind === "sceneEvent") {
    return (
      <div className="flex justify-center py-2">
        <div className={cn(
          "max-w-prose rounded-lg border px-4 py-2 text-center text-xs font-medium",
          kind === "actAnnouncement" ? "border-primary/30 bg-primary/5 text-primary" : "border-amber-500/30 bg-amber-500/5 text-amber-600 dark:text-amber-400",
        )}>
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
              onClick={() => wikiArticleUrls?.[0] && openExternalUrl(wikiArticleUrls[0]).catch(() => {})}
            >
              <BookOpen className="h-3 w-3" />
            </span>
          )}
          {(ragChunkCount ?? 0) > 0 && (
            <span
              className="inline-flex items-center gap-1 rounded-full bg-purple-500/10 px-1.5 py-0.5 text-[10px] font-medium text-purple-500"
              title={
                ragChunkDetails?.length
                  ? ragChunkDetails.map((c) => `${c.fileName} #${c.chunkIndex + 1}: ${c.preview}`).join("\n")
                  : `RAG: ${ragChunkCount} chunk(s)`
              }
            >
              <Database className="h-3 w-3" />
              {ragChunkCount}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {sources && sources.length > 0 && (
            <button
              onClick={() => setShowSources(!showSources)}
              aria-expanded={showSources}
              className="flex items-center gap-1 rounded px-1.5 py-0.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
              title={t("sources.title")}
            >
              <Link2 className="h-3 w-3" />
              {sources.length}
            </button>
          )}
          {message.innerThought && (
            <button
              onClick={() => setShowThought(!showThought)}
              className="flex items-center gap-1 rounded px-1.5 py-0.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <Brain className="h-3 w-3" />
              {message.thoughtKind === "reasoning"
                ? (showThought ? t("arena.hideReasoning") : t("arena.showReasoning"))
                : (showThought ? t("arena.hideThoughts") : t("arena.showThoughts"))}
            </button>
          )}
        </div>
      </div>

      {showSources && sources && sources.length > 0 && (
        <ul className="mb-3 divide-y divide-border/60 rounded-md border border-dashed border-border bg-muted/30 px-3">
          {sources.map((s, i) => <SourceRow key={`${s.url}-${i}`} source={s} />)}
        </ul>
      )}

      {showThought && message.innerThought && (
        <div className="mb-3 rounded-md border border-dashed border-border bg-muted/30 p-3">
          {message.thoughtKind === "reasoning" && (
            <p className="mb-1 text-[10px] font-medium uppercase tracking-wide text-primary">{t("arena.modelReasoning")}</p>
          )}
          <p className="whitespace-pre-wrap text-xs italic text-muted-foreground">
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

      {audience && (
        <ReactionBar
          onReact={audience.onReact}
          used={(message.reactions ?? []).filter((r) => r.fromSpeakerId === "user").length}
          max={audience.max}
        />
      )}

      {(message.reactions?.length ?? 0) > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5 border-t border-border pt-2">
          {(message.reactions ?? []).map((r, i) => {
            const reactorEmoji = emojiMap?.get(r.fromSpeakerId) ?? "";
            const title = [r.justification, r.quote ? `« ${r.quote} »` : null].filter(Boolean).join("\n");
            return (
              <span
                key={i}
                className={cn(
                  "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] motion-safe:animate-in motion-safe:fade-in",
                  REACTION_TONE_CLASS[reactionTone(r.reactionType)],
                  r.pending && "opacity-50",
                )}
                title={title || undefined}
              >
                {REACTION_EMOJI[r.reactionType] ?? "👍"}
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

/** Characters of live reasoning kept on screen (the tail — the bubble must not grow unbounded). */
const REASONING_PREVIEW_CHARS = 600;
/** Refresh period of the "thinking for N s" chronometer. */
const REASONING_CLOCK_MS = 1000;

/** Seconds elapsed since the component mounted (the first reasoning chunk), refreshed every second. */
function useElapsedSeconds(): number {
  const [startedAt] = useState(() => Date.now());
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const id = window.setInterval(() => setSeconds(Math.floor((Date.now() - startedAt) / 1000)), REASONING_CLOCK_MS);
    return () => window.clearInterval(id);
  }, [startedAt]);
  return seconds;
}

/** "réfléchit depuis N s" — mounted only while a reasoning stream is open. */
function ReasoningClock() {
  const { t } = useTranslation();
  const seconds = useElapsedSeconds();
  return <span className="tabular-nums text-muted-foreground">{t("arena.reasoningSince", { seconds })}</span>;
}

export function StreamingBubble({
  speakerName,
  role,
  content,
  emoji,
  participantNames = EMPTY_NAMES,
  variant = "content",
  intention,
}: {
  speakerName: string;
  role: SpeakerRole;
  content: string;
  emoji?: string;
  participantNames?: string[];
  /** "reasoning": the model is still thinking — muted tail preview */
  variant?: "content" | "reasoning";
  /** Backstage: the speaker's declared intention, shown while they think */
  intention?: IntentionData;
}) {
  const { t } = useTranslation();
  const highlighted = useMemo(
    () => (variant === "content" ? highlightNames(content, participantNames) : null),
    [content, participantNames, variant],
  );
  if (variant === "reasoning") {
    const tail = content.length > REASONING_PREVIEW_CHARS ? `…${content.slice(-REASONING_PREVIEW_CHARS)}` : content;
    return (
      <div className="rounded-lg border border-dashed border-border bg-muted/30 p-3 motion-safe:animate-in motion-safe:fade-in">
        <div className="mb-1.5 flex items-center gap-2">
          <SpeakerBadge name={speakerName} role={role} active emoji={emoji} />
          <span className="flex items-center gap-1 text-[10px] font-medium uppercase tracking-wide text-primary">
            <Brain className="h-3 w-3 animate-pulse" />
            {t("arena.reasoningLive")}
          </span>
          <span className="ml-auto text-[10px]"><ReasoningClock /></span>
        </div>
        {intention && (
          <p className="mb-1 text-[11px] text-muted-foreground">
            <span className="font-medium text-foreground">{t(`directive.goals.${intention.goal}`, intention.goal)}</span>
            {" · "}
            {intention.target ?? t("directive.intentionTopic")}
            {intention.angle && <> — {intention.angle}</>}
          </p>
        )}
        <p className="whitespace-pre-wrap text-xs italic text-muted-foreground">{tail}</p>
      </div>
    );
  }
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
