import { describe, expect, it } from "vitest";
import { renderDiscussionHtml, type ExportLabels } from "./export-html";
import type { DiscussionReportData } from "@/components/report/DiscussionReportView";
import { emptyReport } from "./report";

const labels: ExportLabels = {
  appName: "AIrena", synthesis: "Synthèse", discussion: "Discussion", participants: "Participants", turn: "Tour", turns: "Tours", model: "Modèle", date: "Date",
  sources: "Sources", cited: "citée", positions: "Positions", agendas: "Agendas", agendaObjective: "Objectif", agendaRedLine: "Ligne rouge", agendaVictory: "Victoire",
  achieved: "atteint", notAchieved: "non atteint", unknown: "incertain", outcome: "Issue", outcomeVerdict: "Verdict", outcomeAgreement: "Accord", outcomeVote: "Vote",
  argumentMap: "Carte", argumentMapBySpeaker: "Par orateur", document: "Document", reactions: "Réactions",
};

const data: DiscussionReportData = {
  topic: "L'IA <remplace> tout ?",
  discussionMode: "trial",
  turns: 2,
  modelName: "ollama · mistral",
  createdAt: "2026-09-16T10:00:00Z",
  participants: [{ id: "g1", name: "Alice", role: "GladIAteur", emoji: "🦉" }],
  usage: null,
  synthesis: "## Bilan\n\nUne **transformation**.",
  messages: [
    { id: "m1", discussionId: "d", turnNumber: 1, speakerId: "arb", speakerName: "Le Juge", role: "IArbitre", content: "Ouverture", innerThought: null, reactions: [], isBanNotification: false, timestamp: "" },
    { id: "m2", discussionId: "d", turnNumber: 1, speakerId: "g1", speakerName: "Alice", role: "GladIAteur", content: "Les données *montrent*", innerThought: null, reactions: [{ fromSpeakerId: "g2", fromSpeakerName: "Bob", reactionType: "like", targetMessageId: "m2" }], isBanNotification: false, timestamp: "" },
    { id: "m3", discussionId: "d", turnNumber: 2, speakerId: "g1", speakerName: "Alice", role: "GladIAteur", content: "repose ses notes", innerThought: null, reactions: [], isBanNotification: false, kind: "stageDirection", timestamp: "" },
  ],
  argumentMapMd: "# Sujet\n\n## Thèse (Alice)\n- Arg",
  argumentMapMdBySpeaker: "",
  argumentMap: null,
  documentContent: "",
  documentFormat: "none",
  report: {
    ...emptyReport(),
    outcome: { kind: "verdict", votes: [{ voterId: "j", voterName: "Le Juré", choice: "prosecution", reason: "les preuves" }], winner: "prosecution", byArbitre: false },
    agendas: [{ speakerId: "g1", speakerName: "Alice", objective: "faire admettre le coût", redLine: "", victory: "", achieved: true }],
    positions: [{ participantName: "Alice", stance: "prudente", initialStance: "hostile", shift: "a écouté", wouldChangeIf: null }],
    sources: [{ kind: "web", turn: 1, speakerId: "g1", speakerName: "Alice", messageId: "m2", title: "Étude", url: "https://example.org/x", snippet: "", cited: true }, { kind: "rag", turn: 2, speakerId: "g1", speakerName: "Alice", messageId: null, title: "doc.pdf#3", url: "doc.pdf#3", snippet: "", cited: false }],
  },
};

describe("HTML export", () => {
  it("renders a self-contained page with every section, escaped and styled", () => {
    const html = renderDiscussionHtml({ data, labels, svgByThesis: "<svg><g>map</g></svg>", lang: "fr" });
    expect(html.startsWith("<!doctype html>")).toBe(true);
    expect(html).toContain("<style>");
    expect(html).not.toContain("<script");
    // Escaped topic, rendered markdown, turn dividers, roles and reactions
    expect(html).toContain("L'IA &lt;remplace&gt; tout ?");
    expect(html).toContain("<strong>transformation</strong>");
    expect(html).toContain("Tour 1");
    expect(html).toContain("Tour 2");
    expect(html).toContain('class="msg arbitre"');
    expect(html).toContain('class="msg note"');
    expect(html).toContain("Réactions : Bob (like)");
    // Outcome, agendas, positions, sources, map
    expect(html).toContain("Verdict");
    expect(html).toContain("Le Juré");
    expect(html).toContain("faire admettre le coût");
    expect(html).toContain("<s>hostile</s> → prudente");
    expect(html).toContain('<a href="https://example.org/x">Étude</a>');
    expect(html).toContain("doc.pdf#3");
    expect(html).not.toContain('href="doc.pdf#3"');
    expect(html).toContain("<svg><g>map</g></svg>");
  });

  it("never embeds scripts or event handlers coming from the rendered maps", () => {
    const svg = '<svg onload="alert(1)"><script>alert(2)</script><g onclick="x()">safe<a href="javascript:evil()">l</a></g></svg>';
    const html = renderDiscussionHtml({ data, labels, svgByThesis: svg, svgBySpeaker: svg, lang: "fr" });
    expect(html).not.toContain("<script");
    expect(html).not.toContain("onload=");
    expect(html).not.toContain("onclick=");
    expect(html).not.toContain("javascript:");
    expect(html).toContain("<g>safe");
  });

  it("falls back to the markdown outline without SVG and skips empty sections", () => {
    const html = renderDiscussionHtml({ data: { ...data, report: emptyReport(), synthesis: "", messages: [] }, labels, lang: "en" });
    expect(html).toContain("Thèse (Alice)");
    expect(html).not.toContain("Synthèse</h2>");
    expect(html).not.toContain("Discussion</h2>");
    expect(html).not.toContain("Sources</h2>");
    expect(html).toContain('lang="en"');
  });
});
