import { describe, expect, it } from "vitest";
import { computeDocumentDiff } from "./document-diff";

describe("computeDocumentDiff", () => {
  it("returns null without a previous version, when unchanged, or for format none", () => {
    expect(computeDocumentDiff(null, "a", "txt")).toBeNull();
    expect(computeDocumentDiff("same", "same", "md")).toBeNull();
    expect(computeDocumentDiff("a", "b", "none")).toBeNull();
  });

  it("highlights added words in plain text and drops removed ones", () => {
    const diff = computeDocumentDiff("le chat dort", "le chat noir dort", "txt");
    expect(diff?.format).toBe("txt");
    if (diff?.format !== "txt") throw new Error("unreachable");
    const highlighted = diff.segments.filter((s) => s.highlighted).map((s) => s.text.trim());
    expect(highlighted).toEqual(["noir"]);
    expect(diff.segments.map((s) => s.text).join("")).toBe("le chat noir dort");
    // Pure deletion → nothing to highlight
    expect(computeDocumentDiff("le chat noir dort", "le chat dort", "txt")).toBeNull();
  });

  it("reports changed line indices for markdown", () => {
    const prev = "# Titre\n\n- a\n- b";
    const curr = "# Titre\n\n- a\n- b modifié\n- c";
    const diff = computeDocumentDiff(prev, curr, "md");
    if (diff?.format !== "md") throw new Error("expected md diff");
    expect([...diff.changedLines].sort()).toEqual([3, 4]);
  });

  it("reports changed cells for csv as row,col keys", () => {
    const prev = "nom;age\nalice;30\nbob;40";
    const curr = "nom;age\nalice;31\nbob;40\ncarol;50";
    const diff = computeDocumentDiff(prev, curr, "csv");
    if (diff?.format !== "csv") throw new Error("expected csv diff");
    expect(diff.changedCells.has("1,1")).toBe(true);
    expect(diff.changedCells.has("3,0")).toBe(true);
    expect(diff.changedCells.has("2,0")).toBe(false);
  });
});
