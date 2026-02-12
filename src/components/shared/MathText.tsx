import { type ReactNode } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

/** Regex matching \(...\) inline and \[...\] display math delimiters. */
const MATH_PATTERN = /\\\([\s\S]*?\\\)|\\\[[\s\S]*?\\\]/g;

/** Fast check — avoids regex work for the 99% of messages with no math. */
function hasMath(text: string): boolean {
  return text.includes("\\(") || text.includes("\\[");
}

/**
 * Renders a text string with inline/display LaTeX math via KaTeX.
 * Non-math text is returned as-is. Invalid LaTeX falls back to raw text.
 */
export function MathText({ text }: { text: string }): ReactNode {
  if (!hasMath(text)) return <>{text}</>;

  const parts: ReactNode[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(MATH_PATTERN)) {
    const matchStart = match.index;
    // Text segment before this math expression
    if (matchStart > lastIndex) {
      parts.push(text.slice(lastIndex, matchStart));
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

  // Trailing text after the last math expression
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return <>{parts}</>;
}
