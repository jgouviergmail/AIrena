import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { computeDocumentDiff, type DiffResult } from "@/lib/document-diff";
import { SimpleMd } from "@/components/shared/SimpleMd";
import { MathText } from "@/components/shared/MathText";
import { useArenaStore } from "@/stores/useArenaStore";

function CsvTable({ csv, diffResult }: { csv: string; diffResult: DiffResult }) {
  const rows = csv
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => line.split(";"));
  if (rows.length === 0) return null;
  const [header, ...body] = rows;

  const isHighlighted = (row: number, col: number): boolean => {
    if (!diffResult || diffResult.format !== "csv") return false;
    return diffResult.changedCells.has(`${row},${col}`);
  };

  return (
    <div className="overflow-auto">
      <table className="w-full border-collapse text-xs">
        <thead>
          <tr>
            {header.map((cell, i) => (
              <th
                key={i}
                className={cn(
                  "border border-border bg-muted px-2 py-1 text-left font-medium text-foreground",
                  isHighlighted(0, i) && "bg-diff-highlight",
                )}
              >
                {cell.trim()}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {body.map((row, i) => (
            <tr key={i}>
              {row.map((cell, j) => (
                <td
                  key={j}
                  className={cn(
                    "border border-border px-2 py-1 text-muted-foreground",
                    isHighlighted(i + 1, j) && "bg-diff-highlight text-foreground",
                  )}
                >
                  {cell.trim()}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function TxtWithDiff({ text, diffResult }: { text: string; diffResult: DiffResult }) {
  if (!diffResult || diffResult.format !== "txt") {
    return (
      <pre className="whitespace-pre-wrap font-mono text-sm text-foreground">
        <MathText text={text} />
      </pre>
    );
  }
  return (
    <pre className="whitespace-pre-wrap font-mono text-sm text-foreground">
      {diffResult.segments.map((seg, i) =>
        seg.highlighted ? (
          <mark key={i} className="bg-diff-highlight text-foreground">
            <MathText text={seg.text} />
          </mark>
        ) : (
          <MathText key={i} text={seg.text} />
        ),
      )}
    </pre>
  );
}

/** Co-construction document panel content (diff-highlighted rendering). */
export function DocumentPanel() {
  const { t } = useTranslation();
  const documentContent = useArenaStore((s) => s.documentContent);
  const previousDocumentContent = useArenaStore((s) => s.previousDocumentContent);
  const documentFormat = useArenaStore((s) => s.documentFormat);
  const documentLastEditor = useArenaStore((s) => s.documentLastEditor);

  const diffResult = useMemo(
    () => computeDocumentDiff(previousDocumentContent, documentContent, documentFormat),
    [previousDocumentContent, documentContent, documentFormat],
  );

  const formatBadge = documentFormat !== "none" ? `.${documentFormat}` : "";

  return (
    <>
      {/* Format + last editor */}
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-border px-3 py-1">
        <span className="truncate text-[10px] text-muted-foreground">
          {documentLastEditor ? t("document.lastEditedBy", { name: documentLastEditor }) : t("document.awaiting")}
        </span>
        {formatBadge && (
          <span className="shrink-0 rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
            {formatBadge}
          </span>
        )}
      </div>

      {/* Document content */}
      <div className="flex-1 overflow-y-auto px-3 py-3">
        {!documentContent ? (
          <p className="text-sm text-muted-foreground italic">
            {t("document.empty")}
          </p>
        ) : documentFormat === "csv" ? (
          <CsvTable csv={documentContent} diffResult={diffResult} />
        ) : documentFormat === "md" ? (
          <SimpleMd text={documentContent} diffResult={diffResult} />
        ) : (
          <TxtWithDiff text={documentContent} diffResult={diffResult} />
        )}
      </div>
    </>
  );
}
