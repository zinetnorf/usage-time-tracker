import { describe, expect, test } from "vitest";
import { fillRangeGaps, formatDuration, lastNDays, topApps } from "./format";

describe("formatDuration", () => {
  test("seconds below a minute", () => {
    expect(formatDuration(45)).toBe("45s");
  });

  test("minutes and hours", () => {
    expect(formatDuration(60)).toBe("1m");
    expect(formatDuration(3 * 3600 + 5 * 60)).toBe("3h 05m");
    expect(formatDuration(0)).toBe("0s");
  });
});

describe("lastNDays", () => {
  test("returns inclusive range ending today", () => {
    const days = lastNDays(7, new Date(2026, 5, 10)); // 10 jun 2026
    expect(days[0]).toBe("2026-06-04");
    expect(days[6]).toBe("2026-06-10");
    expect(days).toHaveLength(7);
  });

  test("crosses month boundary", () => {
    const days = lastNDays(3, new Date(2026, 6, 1)); // 1 jul 2026
    expect(days).toEqual(["2026-06-29", "2026-06-30", "2026-07-01"]);
  });
});

describe("fillRangeGaps", () => {
  test("fills missing days with zeros preserving data", () => {
    const rows = [{ day: "2026-06-05", active_sec: 100, idle_sec: 20 }];
    const filled = fillRangeGaps(["2026-06-04", "2026-06-05", "2026-06-06"], rows);
    expect(filled).toEqual([
      { day: "2026-06-04", active_sec: 0, idle_sec: 0 },
      { day: "2026-06-05", active_sec: 100, idle_sec: 20 },
      { day: "2026-06-06", active_sec: 0, idle_sec: 0 },
    ]);
  });
});

describe("topApps", () => {
  const row = (name: string, active: number, idle: number) => ({
    app_id: 0,
    display_name: name,
    active_sec: active,
    idle_sec: idle,
  });

  test("keeps top N and groups rest as others", () => {
    const rows = [row("A", 300, 0), row("B", 200, 0), row("C", 50, 10), row("D", 30, 0)];
    const { top, others } = topApps(rows, 2, true);
    expect(top.map((r) => r.display_name)).toEqual(["A", "B"]);
    expect(others).toEqual({ active_sec: 80, idle_sec: 10 });
  });

  test("countIdle=false sorts by active only", () => {
    const rows = [row("A", 10, 500), row("B", 20, 0)];
    const { top } = topApps(rows, 2, false);
    expect(top.map((r) => r.display_name)).toEqual(["B", "A"]);
  });

  test("no others when list fits", () => {
    const { top, others } = topApps([row("A", 10, 0)], 5, true);
    expect(top).toHaveLength(1);
    expect(others).toBeNull();
  });
});
