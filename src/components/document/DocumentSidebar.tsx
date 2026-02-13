import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight, FileText } from "lucide-react";
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

export function DocumentSidebar({ width = 350 }: { width?: number }) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const documentContent = useArenaStore((s) => s.documentContent);
  const previousDocumentContent = useArenaStore((s) => s.previousDocumentContent);
  const documentFormat = useArenaStore((s) => s.documentFormat);
  const documentLastEditor = useArenaStore((s) => s.documentLastEditor);

  const diffResult = useMemo(
    () => computeDocumentDiff(previousDocumentContent, documentContent, documentFormat),
    [previousDocumentContent, documentContent, documentFormat],
  );

  if (isCollapsed) {
    return (
      <div className="flex w-8 flex-col items-center border-l border-border bg-card pt-2">
        <button
          onClick={() => setIsCollapsed(false)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
          title={t("document.expand")}
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="mt-2 text-xs [writing-mode:vertical-lr] text-muted-foreground">
          {t("document.title")}
        </span>
      </div>
    );
  }

  const formatBadge = documentFormat !== "none" ? `.${documentFormat}` : "";

  return (
    <div
      className="flex flex-col border-l border-border bg-card"
      style={{ width }}
    >
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-primary" />
          <span className="text-sm font-medium text-foreground">
            {t("document.title")}
          </span>
          {formatBadge && (
            <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
              {formatBadge}
            </span>
          )}
        </div>
        <button
          onClick={() => setIsCollapsed(true)}
          className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>

      {/* Last editor badge */}
      {documentLastEditor && (
        <div className="border-b border-border px-3 py-1">
          <span className="text-[10px] text-muted-foreground">
            {t("document.lastEditedBy", { name: documentLastEditor })}
          </span>
        </div>
      )}

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
    </div>
  );
}
