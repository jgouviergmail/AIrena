import { describe, expect, it } from "vitest";
import { countNodesBySpeaker } from "./MindmapSidebar";
import type { ArgumentMap } from "@/lib/types";

const map: ArgumentMap = {
  theses: [
    {
      id: "t-0", label: "T", speakerId: "a", speakerName: "Alice",
      arguments: [
        { id: "a-0", label: "x", argType: "support", speakerId: "a", speakerName: "Alice", targetsThesisId: null,
          children: [{ id: "a-1", label: "y", argType: "counter", speakerId: "b", speakerName: "Bob", targetsThesisId: "t-0", children: [] }] },
      ],
    },
    { id: "t-1", label: "T2", speakerId: "b", speakerName: "Bob", arguments: [] },
  ],
};

describe("countNodesBySpeaker", () => {
  it("counts theses and nested arguments per speaker, most active first", () => {
    expect(countNodesBySpeaker(map)).toEqual([
      { name: "Alice", count: 2 },
      { name: "Bob", count: 2 },
    ].sort((x, y) => y.count - x.count));
    expect(countNodesBySpeaker(null)).toEqual([]);
  });
});
