import { describe, expect, it } from "vitest";
import type { SourceRecord } from "./report";
import { domainOf, fromRagChunk, fromWebResult, groupByTurnAndSpeaker, groupByUrl, isCited, isOpenableUrl, sourcesToMarkdown } from "./sources";

const web = fromWebResult(
  { title: "Rapport sur l'automatisation", url: "https://www.example.org/rapport", domain: "www.example.org", snippet: "L'automatisation transforme les métiers techniques sans les supprimer, selon plusieurs études récentes." },
  2, "g1", "Le Scientifique",
);

describe("sources", () => {
  it("detects citations by domain, title or vocabulary and refuses paraphrases", () => {
    expect(isCited("Comme le montre example.org, rien ne disparaît.", web)).toBe(true);
    expect(isCited("Le rapport sur l'automatisation est clair.", web)).toBe(true);
    expect(isCited("L'automatisation transforme les métiers techniques, plusieurs études récentes le confirment.", web)).toBe(true);
    expect(isCited("Parlons plutôt de la météo de demain.", web)).toBe(false);
    expect(isCited("", web)).toBe(false);
    const rag = fromRagChunk({ fileName: "contrat.pdf", chunkIndex: 2, preview: "clause de non concurrence pendant douze mois", relevanceScore: 0.9 }, 1, "g2", "Le Juriste");
    expect(rag.url).toBe("contrat.pdf#3");
    expect(isCited("contrat.pdf dit autre chose", rag)).toBe(false); // pseudo-urls never match by domain
    expect(isOpenableUrl(rag.url)).toBe(false);
    expect(isOpenableUrl(web.url)).toBe(true);
    expect(isOpenableUrl("file:///C:/x")).toBe(false);
    expect(domainOf("https://www.example.org/a")).toBe("example.org");
    expect(domainOf("garbage")).toBe("");
  });

  it("groups by turn/speaker and by url, and exports markdown", () => {
    const rec = (turn: number, speakerId: string, url: string, cited = false): SourceRecord => ({
      kind: "web", turn, speakerId, speakerName: speakerId.toUpperCase(), messageId: "m", title: `T ${url}`, url, snippet: "", cited,
    });
    const list = [rec(1, "a", "https://x/1"), rec(1, "a", "https://x/2", true), rec(1, "b", "https://x/1"), rec(2, "a", "https://x/3")];
    const groups = groupByTurnAndSpeaker(list);
    expect(groups.map((g) => [g.turn, g.speakerId, g.sources.length])).toEqual([[1, "a", 2], [1, "b", 1], [2, "a", 1]]);
    const byUrl = groupByUrl(list);
    expect(byUrl).toHaveLength(3);
    expect(byUrl[0].usedBy).toEqual(["A", "B"]);
    expect(byUrl[1].cited).toBe(true);
    const md = sourcesToMarkdown("Sujet", list, { title: "Sources", turn: "Tour", cited: "citée" });
    expect(md).toContain("# Sources — Sujet");
    expect(md).toContain("## Tour 1 — A");
    expect(md).toContain("[T https://x/2](https://x/2) — citée");
  });
});
