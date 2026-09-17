import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, Check, Copy, Database, ExternalLink, Globe, Quote } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { groupByTurnAndSpeaker, groupByUrl, isOpenableUrl } from "@/lib/sources";
import { openExternalUrl } from "@/lib/tauri-api";
import { cn } from "@/lib/utils";
import type { SourceKind, SourceRecord } from "@/lib/report";

export const SOURCE_KIND_ICON: Record<SourceKind, LucideIcon> = { web: Globe, wiki: BookOpen, rag: Database };
const SOURCE_KIND_CLASS: Record<SourceKind, string> = {
  web: "bg-blue-500/10 text-blue-500",
  wiki: "bg-green-500/10 text-green-500",
  rag: "bg-purple-500/10 text-purple-500",
};
const COPIED_MS = 1500;

type KindFilter = "all" | SourceKind;
type Grouping = "turn" | "url";

/** One source line: kind icon, title (link when openable), snippet, cited mark, copy. */
export function SourceRow({ source, usedBy }: { source: SourceRecord; usedBy?: string[] }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const Icon = SOURCE_KIND_ICON[source.kind];
  const openable = isOpenableUrl(source.url);

  const copy = () => {
    navigator.clipboard?.writeText(source.url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), COPIED_MS);
    }).catch(() => {});
  };

  return (
    <li className="flex items-start gap-2 py-1.5">
      <span className={cn("mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full", SOURCE_KIND_CLASS[source.kind])} title={t(`sources.kind_${source.kind}`)}>
        <Icon className="h-3 w-3" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
          {openable ? (
            <button
              onClick={() => openExternalUrl(source.url).catch(() => {})}
              className="inline-flex max-w-full items-center gap-1 truncate text-left text-xs font-medium text-primary hover:underline"
              title={source.url}
            >
              <span className="truncate">{source.title}</span>
              <ExternalLink className="h-3 w-3 shrink-0" />
            </button>
          ) : (
            <span className="truncate text-xs font-medium text-foreground" title={source.url}>{source.title}</span>
          )}
          {source.cited && (
            <span className="inline-flex items-center gap-0.5 rounded-full bg-amber-500/10 px-1.5 text-[9px] font-medium text-amber-600 dark:text-amber-400" title={t("sources.citedHint")}>
              <Quote className="h-2.5 w-2.5" />
              {t("sources.cited")}
            </span>
          )}
        </div>
        {source.snippet && <p className="line-clamp-2 text-[11px] text-muted-foreground">{source.snippet}</p>}
        {usedBy && usedBy.length > 0 && (
          <p className="text-[10px] text-muted-foreground">{t("sources.usedBy", { names: usedBy.join(", ") })}</p>
        )}
      </div>
      {openable && (
        <button onClick={copy} className="shrink-0 rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground" title={t("sources.copy")}>
          {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
        </button>
      )}
    </li>
  );
}

/** Filterable, groupable list of sources (arena tab and report tab). */
export function SourcesList({ sources, compact = false }: { sources: SourceRecord[]; compact?: boolean }) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<KindFilter>("all");
  const [citedOnly, setCitedOnly] = useState(false);
  const [grouping, setGrouping] = useState<Grouping>("turn");

  const kindsPresent = useMemo(() => new Set(sources.map((s) => s.kind)), [sources]);
  const filtered = useMemo(
    () => sources.filter((s) => (kind === "all" || s.kind === kind) && (!citedOnly || s.cited)),
    [sources, kind, citedOnly],
  );

  if (sources.length === 0) {
    return <p className="p-4 text-center text-sm italic text-muted-foreground">{t("sources.empty")}</p>;
  }

  const chip = (active: boolean, onClick: () => void, label: React.ReactNode, title?: string) => (
    <button
      onClick={onClick}
      title={title}
      aria-pressed={active}
      className={cn(
        "rounded-full border px-2 py-0.5 text-[10px] font-medium transition-colors",
        active ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-accent",
      )}
    >
      {label}
    </button>
  );

  return (
    <div className={cn("flex min-h-0 flex-1 flex-col", compact ? "" : "gap-2")}>
      <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-border px-3 py-1.5">
        {chip(kind === "all", () => setKind("all"), t("sources.filterAll"))}
        {(["web", "wiki", "rag"] as const).filter((k) => kindsPresent.has(k)).map((k) => chip(kind === k, () => setKind(k), t(`sources.kind_${k}`)))}
        <span className="mx-1 h-3 w-px bg-border" />
        {chip(citedOnly, () => setCitedOnly((v) => !v), t("sources.citedOnly"))}
        <span className="ml-auto" />
        {chip(grouping === "turn", () => setGrouping("turn"), t("sources.groupByTurn"))}
        {chip(grouping === "url", () => setGrouping("url"), t("sources.groupByUrl"))}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-1">
        {filtered.length === 0 ? (
          <p className="py-4 text-center text-xs italic text-muted-foreground">{t("sources.noneMatching")}</p>
        ) : grouping === "turn" ? (
          groupByTurnAndSpeaker(filtered).map((g, i) => (
            <div key={`${g.turn}-${g.speakerId}-${i}`} className="mb-2">
              <p className="sticky top-0 bg-card/95 py-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground backdrop-blur">
                {g.turn > 0 ? t("sources.turn", { number: g.turn }) : t("sources.introduction")} · {g.speakerName}
              </p>
              <ul className="divide-y divide-border/60">
                {g.sources.map((s, j) => <SourceRow key={`${s.url}-${j}`} source={s} />)}
              </ul>
            </div>
          ))
        ) : (
          <ul className="divide-y divide-border/60">
            {groupByUrl(filtered).map((e) => <SourceRow key={e.source.url} source={{ ...e.source, cited: e.cited }} usedBy={e.usedBy} />)}
          </ul>
        )}
      </div>
    </div>
  );
}
