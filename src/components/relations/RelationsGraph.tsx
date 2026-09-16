import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { getProfileEmoji } from "@/lib/profile-emoji";
import type { RelationshipEdge } from "@/lib/types";

const SIZE = 300;
const CENTER = SIZE / 2;
const RING = 105;
const NODE_RADIUS = 20;
/** Edge width grows with the number of reactions exchanged, within bounds. */
const EDGE_MIN_WIDTH = 1;
const EDGE_MAX_WIDTH = 6;
const EDGE_WIDTH_PER_REACTION = 0.6;

const EDGE_CLASS: Record<string, string> = {
  ally: "stroke-green-500",
  rival: "stroke-red-500",
  tense: "stroke-amber-500",
};

/** Participants placed on a ring; edges = cumulative reactions (colour = relationship). */
export function RelationsGraph() {
  const { t } = useTranslation();
  const edges = useArenaStore((s) => s.relationships);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);

  const nodes = useMemo(() => {
    const n = Math.max(gladiateurs.length, 1);
    return gladiateurs.map((g, i) => {
      const angle = -Math.PI / 2 + (i * 2 * Math.PI) / n;
      return {
        id: g.id,
        name: g.name,
        emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
        x: CENTER + RING * Math.cos(angle),
        y: CENTER + RING * Math.sin(angle),
      };
    });
  }, [gladiateurs]);
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  // Only edges between drawn participants (reactions aimed at the moderator/user are not part of the graph)
  const drawnEdges = useMemo(() => edges.filter((e) => byId.has(e.a) && byId.has(e.b)), [edges, byId]);

  if (gladiateurs.length < 2) {
    return <p className="p-4 text-center text-sm italic text-muted-foreground">{t("relations.needTwo")}</p>;
  }

  const edgeWidth = (e: RelationshipEdge) => {
    const total = e.abLikes + e.abDislikes + e.baLikes + e.baDislikes;
    return Math.min(EDGE_MAX_WIDTH, EDGE_MIN_WIDTH + total * EDGE_WIDTH_PER_REACTION);
  };

  return (
    <div className="flex flex-1 flex-col overflow-y-auto">
      <svg viewBox={`0 0 ${SIZE} ${SIZE}`} className="mx-auto w-full max-w-sm" role="img" aria-label={t("relations.title")}>
        {drawnEdges.map((e) => {
          const a = byId.get(e.a)!;
          const b = byId.get(e.b)!;
          const cls = e.kind ? EDGE_CLASS[e.kind] : "stroke-muted-foreground/40";
          const title = t("relations.edgeTitle", {
            a: a.name, b: b.name,
            abLikes: e.abLikes, abDislikes: e.abDislikes, baLikes: e.baLikes, baDislikes: e.baDislikes,
          });
          return (
            <g key={`${e.a}-${e.b}`}>
              <title>{title}</title>
              <line
                x1={a.x} y1={a.y} x2={b.x} y2={b.y}
                className={`${cls} motion-safe:transition-all motion-safe:duration-500`}
                strokeWidth={edgeWidth(e)}
                strokeLinecap="round"
                strokeDasharray={e.kind ? undefined : "3 3"}
              />
            </g>
          );
        })}
        {nodes.map((n) => (
          <g key={n.id}>
            <title>{n.name}</title>
            <circle cx={n.x} cy={n.y} r={NODE_RADIUS} className="fill-card stroke-border" strokeWidth={1} />
            <text x={n.x} y={n.y + 1} textAnchor="middle" dominantBaseline="middle" fontSize={16}>{n.emoji}</text>
            <text x={n.x} y={n.y + NODE_RADIUS + 10} textAnchor="middle" className="fill-muted-foreground text-[8px]">
              {n.name.length > 16 ? `${n.name.slice(0, 15)}…` : n.name}
            </text>
          </g>
        ))}
      </svg>

      {/* Legend + edge list */}
      <div className="space-y-1.5 border-t border-border px-3 py-2 text-[10px] text-muted-foreground">
        <div className="flex flex-wrap gap-x-3 gap-y-1">
          <span><span className="inline-block h-1.5 w-4 rounded bg-green-500 align-middle" /> {t("directive.relations.ally")}</span>
          <span><span className="inline-block h-1.5 w-4 rounded bg-red-500 align-middle" /> {t("directive.relations.rival")}</span>
          <span><span className="inline-block h-1.5 w-4 rounded bg-amber-500 align-middle" /> {t("directive.relations.tense")}</span>
          <span><span className="inline-block h-1.5 w-4 rounded bg-muted-foreground/40 align-middle" /> {t("relations.neutral")}</span>
        </div>
        {drawnEdges.length === 0 ? (
          <p className="italic">{t("relations.empty")}</p>
        ) : (
          <ul className="space-y-0.5">
            {drawnEdges.map((e) => (
              <li key={`${e.a}-${e.b}`} className="flex flex-wrap justify-between gap-x-2">
                <span className="truncate">{byId.get(e.a)?.name ?? e.a} ↔ {byId.get(e.b)?.name ?? e.b}</span>
                <span className="shrink-0 font-mono">
                  👍{e.abLikes + e.baLikes} 👎{e.abDislikes + e.baDislikes}
                  {e.kind && <span className="ml-1 text-foreground">· {t(`directive.relations.${e.kind}`)}</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
