import type { ReactElement, ReactNode } from "react";
import { MathText } from "./MathText";
import type { DiffResult } from "@/lib/document-diff";

/** Lightweight markdown: headings (#/##/###), **bold**, *italic*, `code`, ```code blocks```,
 *  bullet/numbered lists (- item / 1. item), horizontal rules (---), tables, paragraphs.
 *  Uses line-by-line block detection to handle mixed content correctly.
 *  Optional diffResult prop enables line-level highlighting of changed blocks. */
export function SimpleMd({ text, diffResult }: { text: string; diffResult?: DiffResult | null }) {
  // Normalize HTML <br> tags to newlines (LLMs sometimes output them instead of \n)
  const normalized = text.replace(/<br\s*\/?>/gi, "\n");

  // Split code blocks first, then process each segment
  const segments = normalized.split(/(```[\s\S]*?```)/g);

  const elements: ReactElement[] = [];
  let key = 0;

  // Diff: extract changedLines set if applicable
  const changedLines = diffResult?.format === "md" ? diffResult.changedLines : null;

  // Pre-compute line offsets for each segment via cumulative newline counting.
  // This correctly maps local line indices to global line indices despite
  // boundary artifacts from the code block regex split.
  const segmentLineOffsets: number[] = [];
  let newlinesSoFar = 0;
  for (const seg of segments) {
    segmentLineOffsets.push(newlinesSoFar);
    newlinesSoFar += (seg.match(/\n/g) || []).length;
  }

  // Helper: wrap an element with a highlight background if any of its source lines changed
  function wrapHighlight(el: ReactElement, lineIndices: number[]): ReactElement {
    if (!changedLines) return el;
    if (!lineIndices.some((idx) => changedLines.has(idx))) return el;
    return (
      <div key={el.key} className="rounded-sm bg-diff-highlight -mx-1 px-1">
        {el}
      </div>
    );
  }

  for (let segIdx = 0; segIdx < segments.length; segIdx++) {
    const segment = segments[segIdx];
    const offset = segmentLineOffsets[segIdx];

    if (segment.startsWith("```") && segment.endsWith("```")) {
      const code = segment.slice(3, -3).replace(/^\w*\n/, ""); // strip optional language hint
      const nbLines = segment.split("\n").length;
      const lineRange = Array.from({ length: nbLines }, (_, idx) => offset + idx);
      const el = (
        <pre key={key++} className="overflow-auto rounded-md bg-muted px-3 py-2 font-mono text-xs text-foreground">
          {code}
        </pre>
      );
      elements.push(wrapHighlight(el, lineRange));
      continue;
    }

    // Process lines in blocks, detecting block boundaries dynamically
    const lines = segment.split("\n");
    let i = 0;

    while (i < lines.length) {
      const line = lines[i];
      const trimmed = line.trim();

      // Skip empty lines
      if (!trimmed) {
        i++;
        continue;
      }

      // Horizontal rule
      if (/^-{3,}$/.test(trimmed) || /^\*{3,}$/.test(trimmed)) {
        const el = <hr key={key++} className="my-2 border-border" />;
        elements.push(wrapHighlight(el, [offset + i]));
        i++;
        continue;
      }

      // Headings (### before ## before #)
      if (trimmed.startsWith("### ")) {
        const el = (
          <h4 key={key++} className="text-sm font-semibold text-foreground">
            <InlineMarkdown text={trimmed.slice(4)} />
          </h4>
        );
        elements.push(wrapHighlight(el, [offset + i]));
        i++;
        continue;
      }
      if (trimmed.startsWith("## ")) {
        const el = (
          <h3 key={key++} className="text-sm font-semibold text-foreground">
            <InlineMarkdown text={trimmed.slice(3)} />
          </h3>
        );
        elements.push(wrapHighlight(el, [offset + i]));
        i++;
        continue;
      }
      if (trimmed.startsWith("# ")) {
        const el = (
          <h2 key={key++} className="text-base font-bold text-foreground">
            <InlineMarkdown text={trimmed.slice(2)} />
          </h2>
        );
        elements.push(wrapHighlight(el, [offset + i]));
        i++;
        continue;
      }

      // Table: collect consecutive pipe-delimited lines
      if (isTableLine(trimmed)) {
        const startI = i;
        const tableLines: string[] = [];
        while (i < lines.length && lines[i].trim() && isTableLine(lines[i].trim())) {
          tableLines.push(lines[i].trim());
          i++;
        }
        const lineRange = Array.from({ length: i - startI }, (_, idx) => offset + startI + idx);
        if (tableLines.length >= 2) {
          const el = <MdTable key={key++} lines={tableLines} />;
          elements.push(wrapHighlight(el, lineRange));
        } else {
          // Single pipe line — treat as paragraph
          const el = (
            <p key={key++} className="leading-relaxed">
              <InlineMarkdown text={tableLines[0]} />
            </p>
          );
          elements.push(wrapHighlight(el, lineRange));
        }
        continue;
      }

      // Bullet list: collect consecutive bullet lines (- or * prefix)
      if (/^[-*]\s/.test(trimmed)) {
        const startI = i;
        const items: string[] = [];
        while (i < lines.length && /^[-*]\s/.test(lines[i].trim())) {
          items.push(lines[i].trim().replace(/^[-*]\s+/, ""));
          i++;
        }
        const lineRange = Array.from({ length: i - startI }, (_, idx) => offset + startI + idx);
        const el = (
          <ul key={key++} className="list-disc space-y-1 pl-5">
            {items.map((item, j) => (
              <li key={j}><InlineMarkdown text={item} /></li>
            ))}
          </ul>
        );
        elements.push(wrapHighlight(el, lineRange));
        continue;
      }

      // Numbered list: collect consecutive numbered lines (1. / 2. etc.)
      if (/^\d+[.)]\s/.test(trimmed)) {
        const startI = i;
        const items: string[] = [];
        while (i < lines.length && /^\d+[.)]\s/.test(lines[i].trim())) {
          items.push(lines[i].trim().replace(/^\d+[.)]\s+/, ""));
          i++;
        }
        const lineRange = Array.from({ length: i - startI }, (_, idx) => offset + startI + idx);
        const el = (
          <ol key={key++} className="list-decimal space-y-1 pl-5">
            {items.map((item, j) => (
              <li key={j}><InlineMarkdown text={item} /></li>
            ))}
          </ol>
        );
        elements.push(wrapHighlight(el, lineRange));
        continue;
      }

      // Paragraph: collect consecutive non-special lines
      const startI = i;
      const paraLines: string[] = [];
      while (
        i < lines.length &&
        lines[i].trim() &&
        !isBlockStart(lines[i].trim())
      ) {
        paraLines.push(lines[i].trim());
        i++;
      }
      if (paraLines.length > 0) {
        const lineRange = Array.from({ length: i - startI }, (_, idx) => offset + startI + idx);
        const el = (
          <p key={key++} className="leading-relaxed">
            <InlineMarkdown text={paraLines.join("\n")} />
          </p>
        );
        elements.push(wrapHighlight(el, lineRange));
      }
    }
  }

  return <div className="space-y-3 text-sm text-foreground">{elements}</div>;
}

/** Check if a line looks like a table row (contains pipe delimiters or is a separator). */
function isTableLine(trimmed: string): boolean {
  return trimmed.startsWith("|") || (trimmed.includes(" | ") && !trimmed.startsWith("#"));
}

/** Check if a line starts a new block (heading, list, hr, table). */
function isBlockStart(trimmed: string): boolean {
  if (!trimmed) return true; // empty line = block boundary
  if (/^#{1,3}\s/.test(trimmed)) return true;
  if (/^-{3,}$/.test(trimmed) || /^\*{3,}$/.test(trimmed)) return true;
  if (/^[-*]\s/.test(trimmed)) return true;
  if (/^\d+[.)]\s/.test(trimmed)) return true;
  if (isTableLine(trimmed)) return true;
  return false;
}

/** Render a markdown table. Handles optional separator row (|---|---|). */
function MdTable({ lines }: { lines: string[] }) {
  const parseRow = (line: string): string[] =>
    line.split("|").map((c) => c.trim()).filter((_, i, arr) => {
      // Remove empty first/last cells from leading/trailing pipes
      if (i === 0 && !arr[0]) return false;
      if (i === arr.length - 1 && !arr[arr.length - 1]) return false;
      return true;
    });

  const isSeparator = (line: string): boolean => /^\|?[\s-:|]+\|?$/.test(line.trim());

  const rows = lines.filter((l) => l.trim().length > 0);
  if (rows.length === 0) return null;

  // First row is header
  const headerCells = parseRow(rows[0]);
  // Skip separator row if present
  const dataStart = rows.length > 1 && isSeparator(rows[1]) ? 2 : 1;
  const dataRows = rows.slice(dataStart).map(parseRow);

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-border">
            {headerCells.map((cell, i) => (
              <th key={i} className="px-3 py-1.5 text-left font-semibold text-foreground">
                <InlineMarkdown text={cell} />
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {dataRows.map((cells, ri) => (
            <tr key={ri} className="border-b border-border/50">
              {cells.map((cell, ci) => (
                <td key={ci} className="px-3 py-1.5 text-foreground">
                  <InlineMarkdown text={cell} />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Inline markdown: **bold**, *italic*, `code`, and LaTeX math */
function InlineMarkdown({ text }: { text: string }) {
  // First split on math delimiters to preserve them, then apply bold/code on non-math segments
  const mathPattern = /(\\\([\s\S]*?\\\)|\\\[[\s\S]*?\\\])/g;
  const mathSegments = text.split(mathPattern);

  const result: ReactNode[] = [];
  let key = 0;

  for (const seg of mathSegments) {
    if ((seg.startsWith("\\(") && seg.endsWith("\\)")) || (seg.startsWith("\\[") && seg.endsWith("\\]"))) {
      // Math segment — render via MathText
      result.push(<MathText key={key++} text={seg} />);
    } else {
      // Non-math: apply bold/italic/code processing
      const parts = seg.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g);
      for (const part of parts) {
        if (part.startsWith("**") && part.endsWith("**")) {
          result.push(<strong key={key++}>{part.slice(2, -2)}</strong>);
        } else if (part.startsWith("*") && part.endsWith("*") && part.length > 2) {
          result.push(<em key={key++}>{part.slice(1, -1)}</em>);
        } else if (part.startsWith("`") && part.endsWith("`")) {
          result.push(
            <code key={key++} className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
              {part.slice(1, -1)}
            </code>,
          );
        } else if (part) {
          result.push(<span key={key++}>{part}</span>);
        }
      }
    }
  }

  return <>{result}</>;
}
