import { describe, expect, it } from "vitest";
import { REPLAY_MAX_GAP_MS, REPLAY_MIN_GAP_MS, replayDuration, scheduleReplay } from "./replay";

describe("scheduleReplay (S48)", () => {
  it("keeps the real deltas divided by the speed, within the bounds", () => {
    const base = Date.parse("2026-09-16T10:00:00Z");
    // 12 messages: 2 s apart, one 60 s silence, one out-of-order stamp, one unparsable
    const stamps = Array.from({ length: 12 }, (_, i) => base + i * 2000);
    stamps[6] = stamps[5] + 60_000;
    for (let i = 7; i < 12; i++) stamps[i] = stamps[6] + (i - 6) * 2000;
    stamps[9] = stamps[8] - 5000;
    const messages = stamps.map((t, i) => ({ timestamp: i === 10 ? "not a date" : new Date(t).toISOString() }));

    const x4 = scheduleReplay(messages, 4);
    expect(x4).toHaveLength(12);
    expect(x4[0].at).toBe(0);
    expect(x4[1].at - x4[0].at).toBe(500);
    // The long silence is capped, then divided
    expect(x4[6].at - x4[5].at).toBe(REPLAY_MAX_GAP_MS / 4);
    // Out of order and unparsable: no delay beyond the floor
    expect(x4[9].at - x4[8].at).toBe(REPLAY_MIN_GAP_MS);
    expect(x4[10].at - x4[9].at).toBe(REPLAY_MIN_GAP_MS);
    // Monotonic
    for (let i = 1; i < x4.length; i++) expect(x4[i].at).toBeGreaterThan(x4[i - 1].at);
    // ×1 is four times slower on regular gaps, ×8 hits the floor
    const x1 = scheduleReplay(messages, 1);
    expect(x1[1].at).toBe(2000);
    const x8 = scheduleReplay(messages, 8);
    expect(x8[1].at).toBe(REPLAY_MIN_GAP_MS);
    expect(replayDuration(x8)).toBe(x8[11].at);
    expect(replayDuration([])).toBe(0);
  });
});
