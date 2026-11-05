import { describe, expect, it } from "vitest";
import { formatAge, formatPercent, formatReset, formatTokens } from "./format";

/** Seconds since the epoch of a local date, so tests hold in any time zone. */
function local(day: number, hour: number, minute = 0): number {
  return new Date(2026, 10, day, hour, minute).getTime() / 1000;
}

describe("formatPercent", () => {
  it("rounds to a whole percent", () => {
    expect(formatPercent(41.4)).toBe("41 %");
    expect(formatPercent(0)).toBe("0 %");
  });
});

describe("formatTokens", () => {
  it("shows millions with one decimal", () => {
    expect(formatTokens(1_200_000)).toBe("1,2 M tokens");
    expect(formatTokens(59_160_902)).toBe("59,2 M tokens");
    expect(formatTokens(3_000_000)).toBe("3 M tokens");
  });

  it("shows thousands, then raw counts", () => {
    expect(formatTokens(850_400)).toBe("850 k tokens");
    expect(formatTokens(12)).toBe("12 tokens");
    expect(formatTokens(0)).toBe("0 tokens");
  });
});

describe("formatReset", () => {
  it("shows the hour of a reset within 24 h", () => {
    expect(formatReset(local(4, 17, 40), local(4, 13))).toBe("17:40");
  });

  it("shows the weekday of a later reset", () => {
    // 2026-11-09 is a Monday.
    expect(formatReset(local(9, 1, 36), local(4, 13))).toBe("lun.");
  });
});

describe("formatAge", () => {
  const now = local(4, 13);

  it("shows fresh data as just now", () => {
    expect(formatAge(now - 20, now)).toBe("à l'instant");
    expect(formatAge(now + 5, now)).toBe("à l'instant");
  });

  it("shows minutes, hours, then days", () => {
    expect(formatAge(now - 3 * 60, now)).toBe("il y a 3 min");
    expect(formatAge(now - 2 * 3600, now)).toBe("il y a 2 h");
    expect(formatAge(now - 3 * 86400, now)).toBe("il y a 3 j");
  });
});
