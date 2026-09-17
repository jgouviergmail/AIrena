// Standalone HTML export of a finished discussion (v1.20): one self-contained
// file (inline CSS, no script, embedded SVG maps) that opens offline in any
// browser and prints cleanly. Rendered with the same lightweight markdown
// component as the app.
import { renderToStaticMarkup } from "react-dom/server";
import { SimpleMd } from "@/components/shared/SimpleMd";
import type { DiscussionReportData } from "@/components/report/DiscussionReportView";
import type { ModeOutcome } from "./types";

/** Labels the export needs (translated by the caller). */
export interface ExportLabels {
  appName: string;
  synthesis: string;
  discussion: string;
  participants: string;
  turn: string;
  turns: string;
  model: string;
  date: string;
  sources: string;
  cited: string;
  positions: string;
  agendas: string;
  agendaObjective: string;
  agendaRedLine: string;
  agendaVictory: string;
  achieved: string;
  notAchieved: string;
  unknown: string;
  outcome: string;
  outcomeVerdict: string;
  outcomeAgreement: string;
  outcomeVote: string;
  argumentMap: string;
  argumentMapBySpeaker: string;
  document: string;
  reactions: string;
}

export interface ExportInput {
  data: DiscussionReportData;
  labels: ExportLabels;
  /** Rendered SVG of the argument maps, when the viewer had them */
  svgByThesis?: string | null;
  svgBySpeaker?: string | null;
  /** Language attribute of the document */
  lang: string;
}

/** Print-friendly stylesheet: system fonts, one column, page breaks between sections. */
const STYLE = `
:root{color-scheme:light}
body{margin:0;padding:24px;font:15px/1.55 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;color:#1c1c1e;background:#f6f6f7}
main{max-width:860px;margin:0 auto}
h1{font-size:1.6rem;margin:0 0 .25rem}
h2{font-size:1.2rem;margin:2rem 0 .75rem;padding-bottom:.25rem;border-bottom:1px solid #d9d9de}
h3{font-size:1rem;margin:1.25rem 0 .5rem}
.meta{color:#5a5a62;font-size:.9rem}
.card{background:#fff;border:1px solid #e2e2e6;border-radius:10px;padding:16px 20px;margin:12px 0}
.msg{border-left:3px solid #c7c7cf;padding:8px 14px;margin:10px 0;background:#fff;border-radius:0 8px 8px 0}
.msg.arbitre{border-left-color:#7c5cff}
.msg.user{border-left-color:#0aa}
.msg.note{border-left-color:#e8b21a;font-style:italic;color:#5a5a62}
.msg header{font-weight:600;font-size:.9rem;margin-bottom:4px}
.msg .reactions{font-size:.8rem;color:#5a5a62;margin-top:6px}
.turn{margin:1.5rem 0 .5rem;font-size:.8rem;letter-spacing:.08em;text-transform:uppercase;color:#5a5a62}
.badge{display:inline-block;padding:1px 8px;border-radius:999px;font-size:.75rem;background:#eceef5;color:#3b3b7a;margin-left:6px}
.badge.ok{background:#e3f6ea;color:#1e6b3a}.badge.ko{background:#fbe9e2;color:#8a3a1e}
table{border-collapse:collapse;width:100%;font-size:.9rem}th,td{border:1px solid #e2e2e6;padding:6px 8px;text-align:left;vertical-align:top}
ul{padding-left:1.2rem}code{background:#eee;padding:1px 4px;border-radius:4px;font-size:.9em}
pre{background:#f0f0f3;padding:10px;border-radius:8px;overflow-x:auto}
svg{max-width:100%;height:auto;background:#fff;border:1px solid #e2e2e6;border-radius:10px}
.katex-html{display:none}
a{color:#3b3b7a}
@media print{body{background:#fff;padding:0}h2{page-break-after:avoid}.card,.msg{page-break-inside:avoid}}
`;

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function md(text: string): string {
  return renderToStaticMarkup(<SimpleMd text={text} />);
}

/** The exported page carries no script: an SVG rendered from model-written
 * labels is stripped of scripts, event handlers and javascript: links. */
export function sanitizeSvg(svg: string): string {
  return svg
    .replace(/<script[\s\S]*?<\/script\s*>/gi, "")
    .replace(/\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, "")
    .replace(/(href|xlink:href)\s*=\s*("|')\s*javascript:[^"']*("|')/gi, "$1=$2#$3");
}

function outcomeHtml(outcome: ModeOutcome, l: ExportLabels): string {
  if (outcome.kind === "verdict") {
    const rows = outcome.votes.map((v) => `<li><b>${escape(v.voterName)}</b> : ${escape(v.choice)}${v.reason ? ` — ${escape(v.reason)}` : ""}</li>`).join("");
    return `<div class="card"><h3>${escape(l.outcomeVerdict)}</h3><p><b>${escape(outcome.winner ?? "—")}</b>${outcome.byArbitre ? ` <span class="badge">IArbitre</span>` : ""}</p><ul>${rows}</ul></div>`;
  }
  if (outcome.kind === "agreement") {
    const rows = outcome.parties.map((p) => `<li><b>${escape(p.partyName)}</b> : <span class="badge ${p.accepts ? "ok" : "ko"}">${p.accepts ? "✓" : "✗"}</span>${p.reason ? ` ${escape(p.reason)}` : ""}</li>`).join("");
    return `<div class="card"><h3>${escape(l.outcomeAgreement)}</h3><p><span class="badge ${outcome.reached ? "ok" : "ko"}">${outcome.reached ? "✓" : "✗"}</span></p><ul>${rows}</ul></div>`;
  }
  return `<div class="card"><h3>${escape(l.outcomeVote)}</h3><p>${escape(outcome.before ?? "—")} → ${escape(outcome.after ?? "—")}${outcome.winner ? ` <span class="badge ok">${escape(outcome.winner)}</span>` : ""}</p></div>`;
}

/** The whole page as a string (never throws on partial data). */
export function renderDiscussionHtml({ data, labels: l, svgByThesis: rawByThesis, svgBySpeaker: rawBySpeaker, lang }: ExportInput): string {
  const report = data.report;
  const svgByThesis = rawByThesis ? sanitizeSvg(rawByThesis) : rawByThesis;
  const svgBySpeaker = rawBySpeaker ? sanitizeSvg(rawBySpeaker) : rawBySpeaker;
  const participants = data.participants.map((p) => `<span>${escape(p.emoji)} ${escape(p.name)}</span>`).join(" · ");
  const head = `<header class="card"><h1>${escape(data.topic)}</h1><p class="meta">${escape(l.date)} : ${escape(data.createdAt ? new Date(data.createdAt).toLocaleString(lang) : new Date().toLocaleString(lang))} · ${escape(l.turns)} : ${data.turns} · ${escape(l.model)} : ${escape(data.modelName)}</p><p class="meta">${escape(l.participants)} : ${participants}</p></header>`;

  const outcome = report.outcome ? `<section><h2>${escape(l.outcome)}</h2>${outcomeHtml(report.outcome, l)}</section>` : "";
  const synthesis = data.synthesis ? `<section><h2>${escape(l.synthesis)}</h2><div class="card">${md(data.synthesis)}</div></section>` : "";

  const agendas = report.agendas.length > 0
    ? `<section><h2>${escape(l.agendas)}</h2>${report.agendas.map((a) => {
        const verdict = a.achieved === true ? `<span class="badge ok">${escape(l.achieved)}</span>` : a.achieved === false ? `<span class="badge ko">${escape(l.notAchieved)}</span>` : `<span class="badge">${escape(l.unknown)}</span>`;
        return `<div class="card"><b>${escape(a.speakerName)}</b> ${verdict}<ul>${a.objective ? `<li>${escape(l.agendaObjective)} : ${escape(a.objective)}</li>` : ""}${a.redLine ? `<li>${escape(l.agendaRedLine)} : ${escape(a.redLine)}</li>` : ""}${a.victory ? `<li>${escape(l.agendaVictory)} : ${escape(a.victory)}</li>` : ""}</ul></div>`;
      }).join("")}</section>`
    : "";

  const positions = report.positions.length > 0
    ? `<section><h2>${escape(l.positions)}</h2><table><tbody>${report.positions.map((p) => `<tr><th>${escape(p.participantName)}</th><td>${p.initialStance && p.initialStance !== p.stance ? `<s>${escape(p.initialStance)}</s> → ` : ""}${escape(p.stance)}${p.shift ? `<br><small>${escape(p.shift)}</small>` : ""}</td></tr>`).join("")}</tbody></table></section>`
    : "";

  let lastTurn = -1;
  const messages = data.messages.map((m) => {
    const turnHeader = m.turnNumber !== lastTurn ? `<div class="turn">${escape(l.turn)} ${m.turnNumber}</div>` : "";
    lastTurn = m.turnNumber;
    const kind = m.kind ?? (m.isBanNotification ? "banNotification" : "normal");
    const cls = kind !== "normal" ? "note" : m.role === "IArbitre" ? "arbitre" : m.role === "user" ? "user" : "";
    const reactions = m.reactions.length > 0 ? `<div class="reactions">${escape(l.reactions)} : ${m.reactions.map((r) => `${escape(r.fromSpeakerName)} (${escape(r.reactionType)})`).join(", ")}</div>` : "";
    return `${turnHeader}<article class="msg ${cls}"><header>${escape(m.speakerName)}</header>${md(m.content)}${reactions}</article>`;
  }).join("");
  const discussion = data.messages.length > 0 ? `<section><h2>${escape(l.discussion)}</h2>${messages}</section>` : "";

  const maps = data.argumentMapMd
    ? `<section><h2>${escape(l.argumentMap)}</h2>${svgByThesis ? svgByThesis : md(data.argumentMapMd)}${data.argumentMapMdBySpeaker ? `<h3>${escape(l.argumentMapBySpeaker)}</h3>${svgBySpeaker ? svgBySpeaker : md(data.argumentMapMdBySpeaker)}` : ""}</section>`
    : "";

  const document = data.documentContent && data.documentFormat !== "none"
    ? `<section><h2>${escape(l.document)}</h2><div class="card">${data.documentFormat === "md" ? md(data.documentContent) : `<pre>${escape(data.documentContent)}</pre>`}</div></section>`
    : "";

  const sources = report.sources.length > 0
    ? `<section><h2>${escape(l.sources)}</h2><ul>${report.sources.map((s) => `<li>${escape(l.turn)} ${s.turn} · ${escape(s.speakerName)} : ${/^https?:\/\//.test(s.url) ? `<a href="${escape(s.url)}">${escape(s.title || s.url)}</a>` : escape(s.title || s.url)}${s.cited ? ` <span class="badge ok">${escape(l.cited)}</span>` : ""}</li>`).join("")}</ul></section>`
    : "";

  return `<!doctype html><html lang="${escape(lang)}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(l.appName)} — ${escape(data.topic)}</title><style>${STYLE}</style></head><body><main>${head}${outcome}${synthesis}${agendas}${positions}${discussion}${maps}${document}${sources}</main></body></html>`;
}
