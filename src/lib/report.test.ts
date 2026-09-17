import { describe, expect, it } from "vitest";
import { emptyReport, parseReportJson, reportHasContent, serializeReport, REPORT_VERSION } from "./report";
import { messageKind } from "./types";

describe("report persistence", () => {
  it("parses empty, malformed and partial JSON into safe defaults", () => {
    expect(parseReportJson("")).toEqual(emptyReport());
    expect(parseReportJson(undefined)).toEqual(emptyReport());
    expect(parseReportJson("{not json")).toEqual(emptyReport());
    expect(parseReportJson("null")).toEqual(emptyReport());
    const partial = parseReportJson('{"version":1,"sources":[{"kind":"web","turn":1,"speakerId":"g1","speakerName":"A","messageId":null,"title":"T","url":"https://x","snippet":"","cited":false}],"emotionHistory":"garbage","agendas":null}');
    expect(partial.sources).toHaveLength(1);
    expect(partial.emotionHistory).toEqual({});
    expect(partial.agendas).toEqual([]);
    expect(partial.version).toBe(REPORT_VERSION);
  });

  it("serialises only when there is content (keeps the DB small) and round-trips", () => {
    expect(serializeReport(emptyReport())).toBe("");
    const report = { ...emptyReport(), relationships: [{ a: "g1", b: "g2", abLikes: 2, abDislikes: 0, baLikes: 2, baDislikes: 0, kind: "ally" }] };
    expect(reportHasContent(report)).toBe(false);
    const withHistory = { ...report, emotionHistory: { g1: [{ turn: 1, emotions: { engagement: 50, accord: 50, confiance: 50, frustration: 10, curiosite: 50, enthousiasme: 50 } }] } };
    expect(reportHasContent(withHistory)).toBe(true);
    expect(parseReportJson(serializeReport(withHistory))).toEqual(withHistory);
  });

  it("derives the message kind from the v1.16 ban flag", () => {
    expect(messageKind({ isBanNotification: false })).toBe("normal");
    expect(messageKind({ isBanNotification: true })).toBe("banNotification");
    expect(messageKind({ isBanNotification: false, kind: "stageDirection" })).toBe("stageDirection");
  });
});
