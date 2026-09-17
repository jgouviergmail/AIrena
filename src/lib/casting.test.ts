import { describe, expect, it } from "vitest";
import { analyseCast, classifyPair, contrastLevel, socialDistance } from "./casting";
import { modeSupportsHiddenAgenda, selectableRoles } from "./modes";

const persona = (o: number, e: number, a: number) =>
  `<persona><psychology><big_five_matrix>O=${o} C=5 E=${e} A=${a} N=5</big_five_matrix></psychology></persona>`;

describe("casting compatibility", () => {
  it("classifies alike agreeable profiles as allies, distant or frontal ones as friction", () => {
    expect(classifyPair({ O: 7, C: 5, E: 6, A: 7, N: 5 }, { O: 8, C: 5, E: 5, A: 6, N: 5 })).toBe("allies");
    // Alike but disagreeable: not allies
    expect(classifyPair({ O: 7, C: 5, E: 6, A: 4, N: 5 }, { O: 8, C: 5, E: 5, A: 4, N: 5 })).toBe("neutral");
    // Two frontal profiles clash even when alike
    expect(classifyPair({ O: 7, C: 5, E: 6, A: 2, N: 5 }, { O: 7, C: 5, E: 6, A: 3, N: 5 })).toBe("friction");
    // Far apart on the social axes
    expect(classifyPair({ O: 2, C: 5, E: 2, A: 9, N: 5 }, { O: 9, C: 5, E: 9, A: 4, N: 5 })).toBe("friction");
    expect(socialDistance({ O: 1, C: 1, E: 1, A: 1, N: 1 }, { O: 10, C: 10, E: 10, A: 10, N: 10 })).toBe(9);
  });

  it("analyses a cast: every scored pair, the contrast level, the unscored left out", () => {
    const members = [
      { id: "g1", name: "Alpha", systemPrompt: persona(8, 7, 8) },
      { id: "g2", name: "Beta", systemPrompt: persona(7, 6, 7) },
      { id: "g3", name: "Gamma", systemPrompt: persona(2, 2, 2) },
      { id: "g4", name: "Custom", systemPrompt: "Un prompt libre sans scores" },
    ];
    const analysis = analyseCast(members);
    expect(analysis.pairs.map((p) => `${p.a.id}-${p.b.id}:${p.kind}`)).toEqual(["g1-g2:allies", "g1-g3:friction", "g2-g3:friction"]);
    expect(analysis.unscored.map((m) => m.id)).toEqual(["g4"]);
    expect(analysis.contrast).toBeCloseTo((1 + 5.667 + 4.667) / 3, 1);
    expect(analysis.contrastLevel).toBe("medium");
    expect(contrastLevel(1)).toBe("low");
    expect(contrastLevel(4)).toBe("high");
    // Below two scored members: no contrast
    expect(analyseCast(members.slice(0, 1))).toMatchObject({ pairs: [], contrast: null, contrastLevel: null });
  });

  it("knows which modes take a hidden agenda and which deal roles", () => {
    expect(modeSupportsHiddenAgenda("debate")).toBe(true);
    expect(modeSupportsHiddenAgenda("collaborativeFiction")).toBe(true);
    expect(modeSupportsHiddenAgenda("negotiation")).toBe(true);
    expect(modeSupportsHiddenAgenda("tutorial")).toBe(false);
    expect(modeSupportsHiddenAgenda("sixHats")).toBe(false);
    expect(selectableRoles("trial")).toEqual(["prosecutor", "defense", "witness", "juror"]);
    expect(selectableRoles("oxfordDebate")).toEqual(["for", "against"]);
    expect(selectableRoles("debate")).toEqual([]);
  });
});
