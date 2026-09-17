import { describe, expect, it } from "vitest";
import { spokenParticipants, userHasSpoken } from "./stage";

describe("audience participant (v1.20.2)", () => {
  it("lists the speakers once, the audience member only after they spoke", () => {
    const msgs = [
      { role: "IArbitre", speakerId: "arb", speakerName: "Arb" },
      { role: "GladIAteur", speakerId: "g1", speakerName: "Alice" },
      { role: "GladIAteur", speakerId: "g1", speakerName: "Alice" },
    ];
    expect(userHasSpoken(msgs)).toBe(false);
    expect(spokenParticipants(msgs)).toEqual([{ id: "g1", name: "Alice" }]);
    const withUser = [...msgs, { role: "user", speakerId: "user", speakerName: "Léo" }, { role: "GladIAteur", speakerId: "g2", speakerName: "Bob" }];
    expect(userHasSpoken(withUser)).toBe(true);
    expect(spokenParticipants(withUser)).toEqual([{ id: "g1", name: "Alice" }, { id: "user", name: "Léo" }, { id: "g2", name: "Bob" }]);
  });
});
import { arcPositions, dominantEmotion, neighbourTurn, shortcutAction } from "./stage";

describe("dominantEmotion", () => {
  it("picks the axis furthest from neutral, frustration from its low baseline", () => {
    const flat = { engagement: 50, accord: 50, confiance: 50, frustration: 10, curiosite: 50, enthousiasme: 50 };
    expect(dominantEmotion(flat)).toBeNull();
    expect(dominantEmotion({ ...flat, curiosite: 80 })).toEqual({ axis: "curiosite", intensity: 30 });
    expect(dominantEmotion({ ...flat, engagement: 15, frustration: 50 })).toEqual({ axis: "frustration", intensity: 40 });
    expect(dominantEmotion({ ...flat, accord: 10 })?.axis).toBe("accord");
  });
});

describe("arcPositions", () => {
  it("places everyone inside the box, on an arc opening downward", () => {
    for (const n of [1, 2, 4, 8]) {
      const pts = arcPositions(n, 600, 200);
      expect(pts).toHaveLength(n);
      for (const p of pts) {
        expect(p.x).toBeGreaterThanOrEqual(0);
        expect(p.x).toBeLessThanOrEqual(600);
        expect(p.y).toBeGreaterThanOrEqual(0);
        expect(p.y).toBeLessThanOrEqual(200);
      }
      if (n > 1) {
        expect(pts[0].x).toBeLessThan(pts[n - 1].x);
        // ends of the arc sit lower than the middle
        const mid = pts[Math.floor(n / 2)];
        expect(pts[0].y).toBeGreaterThan(mid.y - 1);
      }
    }
    expect(arcPositions(0, 600, 200)).toEqual([]);
  });
});

describe("shortcutAction", () => {
  it("maps keys and ignores typing or modifiers", () => {
    expect(shortcutAction({ key: " " })).toBe("pauseResume");
    expect(shortcutAction({ key: "I" })).toBe("intervene");
    expect(shortcutAction({ key: "ArrowRight" })).toBe("nextTurn");
    expect(shortcutAction({ key: "?" })).toBe("help");
    expect(shortcutAction({ key: "n" })).toBe("nextSpeaker");
    expect(shortcutAction({ key: "Escape" })).toBe("exitPresentation");
    expect(shortcutAction({ key: "x" })).toBeNull();
    expect(shortcutAction({ key: " ", targetTag: "TEXTAREA" })).toBeNull();
    expect(shortcutAction({ key: " ", targetTag: "BUTTON" })).toBeNull();
    expect(shortcutAction({ key: "p", editable: true })).toBeNull();
    expect(shortcutAction({ key: "p", ctrlKey: true })).toBeNull();
  });

  it("neighbourTurn stays within the played turns", () => {
    expect(neighbourTurn(3, "previousTurn", 5)).toBe(2);
    expect(neighbourTurn(5, "nextTurn", 5)).toBeNull();
    expect(neighbourTurn(1, "previousTurn", 5)).toBeNull();
  });
});
