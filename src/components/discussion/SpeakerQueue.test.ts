import { describe, expect, it } from "vitest";
import { deriveQueue } from "./SpeakerQueue";
import { formatTokens } from "@/lib/cost-estimate";

describe("deriveQueue", () => {
  it("marks spoken, active, waiting and banned speakers in announced order", () => {
    const q = deriveQueue({
      speakerOrder: ["g1", "g2", "g3", "g4"],
      activeSpeakerId: "g2",
      spokenThisTurn: new Set(["g1"]),
      bannedIds: new Set(["g4"]),
      passedIds: [],
    });
    expect(q.map((e) => [e.id, e.state])).toEqual([
      ["g1", "done"], ["g2", "active"], ["g3", "waiting"], ["g4", "banned"],
    ]);
  });

  it("keeps a speaker 'done' once they have spoken even if still flagged active", () => {
    const q = deriveQueue({
      speakerOrder: ["g1"],
      activeSpeakerId: "g1",
      spokenThisTurn: new Set(["g1"]),
      bannedIds: new Set(),
      passedIds: [],
    });
    expect(q[0].state).toBe("done");
  });

  it("appends participants who passed (UserDriven) without duplicating responders", () => {
    const q = deriveQueue({
      speakerOrder: ["g3"],
      activeSpeakerId: null,
      spokenThisTurn: new Set(),
      bannedIds: new Set(),
      passedIds: ["g1", "g2", "g3"],
    });
    expect(q.map((e) => e.id)).toEqual(["g3", "g1", "g2"]);
    expect(q.filter((e) => e.state === "passed")).toHaveLength(2);
  });
});

describe("formatTokens", () => {
  it("compacts large counts", () => {
    expect(formatTokens(950)).toBe("950");
    expect(formatTokens(1_234)).toBe("1.2k");
    expect(formatTokens(12_345)).toBe("12k");
    expect(formatTokens(2_500_000)).toBe("2.5M");
  });
});
