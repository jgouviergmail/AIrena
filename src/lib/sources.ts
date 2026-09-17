// Sources (web results, Wikipedia articles, document chunks) attached to messages.
import type { SourceRecord } from "./report";
import type { RagChunkInfo, WebSourceInfo, WikiSourceInfo } from "./types";

/** Word-set similarity threshold above which a snippet is considered reused by the speaker. */
export const SOURCE_CITED_SIMILARITY = 0.35;
const MIN_TOKEN_CHARS = 4;
const HTTP_SCHEMES = ["http://", "https://"];

/** A source not yet attached to a message (collected while the speaker prepares). */
export type PendingSource = Omit<SourceRecord, "messageId" | "cited">;

export function fromWebResult(r: WebSourceInfo, turn: number, speakerId: string, speakerName: string): PendingSource {
  return { kind: "web", turn, speakerId, speakerName, title: r.title || r.domain, url: r.url, snippet: r.snippet };
}

export function fromWikiArticle(a: WikiSourceInfo, turn: number, speakerId: string, speakerName: string): PendingSource {
  return { kind: "wiki", turn, speakerId, speakerName, title: a.title, url: a.url, snippet: a.snippet };
}

export function fromRagChunk(c: RagChunkInfo, turn: number, speakerId: string, speakerName: string): PendingSource {
  return {
    kind: "rag",
    turn,
    speakerId,
    speakerName,
    title: `${c.fileName} #${c.chunkIndex + 1}`,
    url: `${c.fileName}#${c.chunkIndex + 1}`,
    snippet: c.preview,
  };
}

/** Significant words of a text (lowercase, ≥ 4 chars). */
function tokens(text: string): Set<string> {
  return new Set(
    text
      .toLowerCase()
      .split(/[^\p{L}\p{N}]+/u)
      .filter((w) => w.length >= MIN_TOKEN_CHARS),
  );
}

/** Registrable domain without "www." ("www.example.com" → "example.com"). */
export function domainOf(url: string): string {
  const host = url.split("//")[1]?.split("/")[0] ?? "";
  return host.replace(/^www\./, "");
}

/**
 * Heuristic: the message mentions the source's domain, its title, or reuses
 * enough of the snippet's vocabulary. Never a certainty — the UI says "probably".
 */
export function isCited(messageText: string, source: PendingSource): boolean {
  const text = messageText.toLowerCase();
  if (!text.trim()) return false;
  const domain = source.kind === "rag" ? "" : domainOf(source.url);
  if (domain && text.includes(domain)) return true;
  const title = source.title.toLowerCase().trim();
  if (title.length >= 6 && text.includes(title)) return true;
  const snippetTokens = tokens(source.snippet);
  if (snippetTokens.size < 5) return false;
  const messageTokens = tokens(text);
  let shared = 0;
  for (const w of snippetTokens) if (messageTokens.has(w)) shared += 1;
  return shared / snippetTokens.size >= SOURCE_CITED_SIMILARITY;
}

/** Only web links open in the browser; document pseudo-urls never do. */
export function isOpenableUrl(url: string): boolean {
  return HTTP_SCHEMES.some((s) => url.startsWith(s));
}

export interface SourceGroup {
  turn: number;
  speakerId: string;
  speakerName: string;
  sources: SourceRecord[];
}

/** Group by turn then speaker, keeping the first-seen order. */
export function groupByTurnAndSpeaker(sources: SourceRecord[]): SourceGroup[] {
  const groups: SourceGroup[] = [];
  for (const s of sources) {
    const last = groups[groups.length - 1];
    if (last && last.turn === s.turn && last.speakerId === s.speakerId) {
      last.sources.push(s);
    } else {
      groups.push({ turn: s.turn, speakerId: s.speakerId, speakerName: s.speakerName, sources: [s] });
    }
  }
  return groups;
}

/** One entry per URL, with everyone who used it (for the "group by link" view). */
export function groupByUrl(sources: SourceRecord[]): { source: SourceRecord; usedBy: string[]; cited: boolean }[] {
  const map = new Map<string, { source: SourceRecord; usedBy: string[]; cited: boolean }>();
  for (const s of sources) {
    const entry = map.get(s.url);
    if (entry) {
      if (!entry.usedBy.includes(s.speakerName)) entry.usedBy.push(s.speakerName);
      entry.cited = entry.cited || s.cited;
    } else {
      map.set(s.url, { source: s, usedBy: [s.speakerName], cited: s.cited });
    }
  }
  return [...map.values()];
}

/** Markdown export of the sources, grouped by turn. */
export function sourcesToMarkdown(topic: string, sources: SourceRecord[], labels: { title: string; turn: string; cited: string }): string {
  const lines = [`# ${labels.title} — ${topic}`, ""];
  for (const g of groupByTurnAndSpeaker(sources)) {
    lines.push(`## ${labels.turn} ${g.turn} — ${g.speakerName}`);
    for (const s of g.sources) {
      const link = isOpenableUrl(s.url) ? `[${s.title}](${s.url})` : `${s.title} (${s.url})`;
      lines.push(`- ${link}${s.cited ? ` — ${labels.cited}` : ""}`);
    }
    lines.push("");
  }
  return lines.join("\n");
}
