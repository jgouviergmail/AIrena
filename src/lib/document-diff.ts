import { diffWords, diffLines } from "diff";
import type { DocumentFormat } from "@/lib/types";

/** A segment of text that may or may not be highlighted (for txt format). */
export interface TextDiffSegment {
  text: string;
  highlighted: boolean;
}

/** Discriminated union for format-specific diff results. null = no highlighting. */
export type DiffResult =
  | { format: "txt"; segments: TextDiffSegment[] }
  | { format: "md"; changedLines: Set<number> }
  | { format: "csv"; changedCells: Set<string> }
  | null;

/**
 * Compute a diff between previous and current document content.
 * Returns format-specific diff data for rendering highlights, or null if no highlighting needed.
 */
export function computeDocumentDiff(
  previousContent: string | null,
  currentContent: string,
  format: DocumentFormat,
): DiffResult {
  if (!previousContent) return null;
  if (previousContent === currentContent) return null;

  switch (format) {
    case "txt":
      return computeTxtDiff(previousContent, currentContent);
    case "md":
      return computeMdDiff(previousContent, currentContent);
    case "csv":
      return computeCsvDiff(previousContent, currentContent);
    default:
      return null;
  }
}

/** Word-level diff for plain text. */
function computeTxtDiff(prev: string, curr: string): DiffResult {
  const changes = diffWords(prev, curr);
  const segments: TextDiffSegment[] = [];
  let hasHighlight = false;

  for (const change of changes) {
    if (change.removed) continue;
    if (change.added) hasHighlight = true;
    segments.push({
      text: change.value,
      highlighted: !!change.added,
    });
  }

  if (!hasHighlight) return null;
  return { format: "txt", segments };
}

/** Line-level diff for markdown. Returns a set of 0-based line indices that are new/changed. */
function computeMdDiff(prev: string, curr: string): DiffResult {
  const changes = diffLines(prev, curr);
  const changedLines = new Set<number>();
  let lineIndex = 0;

  for (const change of changes) {
    if (!change.value) continue;
    if (change.removed) continue;

    const newlines = (change.value.match(/\n/g) || []).length;
    const lineCount = change.value.endsWith("\n") ? newlines : newlines + 1;

    if (change.added) {
      for (let i = 0; i < lineCount; i++) {
        changedLines.add(lineIndex + i);
      }
    }

    lineIndex += lineCount;
  }

  if (changedLines.size === 0) return null;
  return { format: "md", changedLines };
}

/** Cell-level diff for CSV. Returns a set of "row,col" keys (0-based) for changed cells. */
function computeCsvDiff(prev: string, curr: string): DiffResult {
  const parseCsvRows = (csv: string): string[][] =>
    csv
      .split("\n")
      .filter((line) => line.trim().length > 0)
      .map((line) => line.split(";").map((cell) => cell.trim()));

  const prevRows = parseCsvRows(prev);
  const currRows = parseCsvRows(curr);
  const changedCells = new Set<string>();

  for (let r = 0; r < currRows.length; r++) {
    const currRow = currRows[r];
    const prevRow = r < prevRows.length ? prevRows[r] : null;

    for (let c = 0; c < currRow.length; c++) {
      const prevCell = prevRow && c < prevRow.length ? prevRow[c] : null;
      if (prevCell === null || prevCell !== currRow[c]) {
        changedCells.add(`${r},${c}`);
      }
    }
  }

  if (changedCells.size === 0) return null;
  return { format: "csv", changedCells };
}
