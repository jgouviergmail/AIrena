import { type ReactNode } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

/** Regex matching \(...\) inline and \[...\] display math delimiters. */
const MATH_PATTERN = /\\\([\s\S]*?\\\)|\\\[[\s\S]*?\\\]/g;

/** Regex matching **bold** and *italic* inline markdown. Bold branch first to avoid
 *  matching the inner `*` of `**...**` as italic delimiters. */
const INLINE_MD_PATTERN = /\*\*(.+?)\*\*|\*(.+?)\*/g;

/** Fast check — avoids regex work for the 99% of messages with no math. */
function hasMath(text: string): boolean {
  return text.includes("\\(") || text.includes("\\[");
}

/** Parse **bold** and *italic* in a plain text string, returning ReactNode[]. */
function formatInlineMarkdown(text: string, keyPrefix: string | number = ""): ReactNode[] {
  if (!text.includes("*")) return [text];

  const parts: ReactNode[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(INLINE_MD_PATTERN)) {
    const matchStart = match.index;
    if (matchStart > lastIndex) {
      parts.push(text.slice(lastIndex, matchStart));
    }

    if (match[1] !== undefined) {
      parts.push(<strong key={`${keyPrefix}b${matchStart}`}>{match[1]}</strong>);
    } else {
      parts.push(<em key={`${keyPrefix}i${matchStart}`}>{match[2]}</em>);
    }

    lastIndex = matchStart + match[0].length;
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts;
}

/**
 * Renders a text string with inline/display LaTeX math via KaTeX,
 * and lightweight markdown (**bold**, *italic*) for non-math segments.
 * Invalid LaTeX falls back to raw text.
 */
export function MathText({ text }: { text: string }): ReactNode {
  if (!hasMath(text)) return <>{formatInlineMarkdown(text)}</>;

  const parts: ReactNode[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(MATH_PATTERN)) {
    const matchStart = match.index;
    // Text segment before this math expression — apply inline markdown
    if (matchStart > lastIndex) {
      parts.push(...formatInlineMarkdown(text.slice(lastIndex, matchStart), matchStart));
    }

    const raw = match[0];
    const isDisplay = raw.startsWith("\\[");
    // Strip delimiters: \( ... \) or \[ ... \]
    const formula = raw.slice(2, -2);

    try {
      const html = katex.renderToString(formula, {
        throwOnError: false,
        displayMode: isDisplay,
      });
      // Always use <span> to stay valid inside <p> elements.
      // Display math gets block-like styling via inline style.
      parts.push(
        <span
          key={matchStart}
          style={isDisplay ? { display: "block", margin: "0.5rem 0", textAlign: "center" } : undefined}
          dangerouslySetInnerHTML={{ __html: html }}
        />,
      );
    } catch {
      // Fallback: render raw text if KaTeX fails
      parts.push(raw);
    }

    lastIndex = matchStart + raw.length;
  }

  // Trailing text after the last math expression — apply inline markdown
  if (lastIndex < text.length) {
    parts.push(...formatInlineMarkdown(text.slice(lastIndex), lastIndex));
  }

  return <>{parts}</>;
}
