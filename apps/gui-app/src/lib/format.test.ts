import { describe, expect, it } from "vitest";
import { DEFAULT_TEMPO_RANGE, TEMPO_RANGE_STEPS } from "./bus-settings";
import {
  effectiveBpm,
  formatPitchPercent,
  formatTempoRange,
  nextTempoRange,
  normToSpeedRatio,
  nudgeSpeed,
  speedRatioToNorm,
} from "./format";

describe("tempo range helpers", () => {
  it("exposes Pioneer/Mixxx cycle steps and default ±6%", () => {
    expect(DEFAULT_TEMPO_RANGE).toBe(0.06);
    expect([...TEMPO_RANGE_STEPS]).toEqual([0.06, 0.1, 0.16, 0.25]);
  });

  it("cycles nextTempoRange with wraparound", () => {
    expect(nextTempoRange(0.06)).toBe(0.1);
    expect(nextTempoRange(0.1)).toBe(0.16);
    expect(nextTempoRange(0.16)).toBe(0.25);
    expect(nextTempoRange(0.25)).toBe(0.06);
    expect(nextTempoRange(0.08)).toBe(0.06);
    expect(nextTempoRange(0.08, [0.08, 0.16])).toBe(0.16);
  });

  it("maps fader ends to ±range and clamps", () => {
    expect(normToSpeedRatio(0.5, 0.06)).toBeCloseTo(1, 5);
    expect(normToSpeedRatio(0, 0.06)).toBeCloseTo(1.06, 5);
    expect(normToSpeedRatio(1, 0.06)).toBeCloseTo(0.94, 5);
    expect(normToSpeedRatio(-1, 0.06)).toBeCloseTo(1.06, 5);
    expect(normToSpeedRatio(2, 0.06)).toBeCloseTo(0.94, 5);
  });

  it("inverts ratio to fader and saturates outside range", () => {
    expect(speedRatioToNorm(1, 0.06)).toBeCloseTo(0.5, 5);
    expect(speedRatioToNorm(1.06, 0.06)).toBeCloseTo(0, 5);
    expect(speedRatioToNorm(0.94, 0.06)).toBeCloseTo(1, 5);
    expect(speedRatioToNorm(1.2, 0.06)).toBe(0);
    expect(speedRatioToNorm(0.5, 0.06)).toBe(1);
  });

  it("computes effective BPM from ratio span", () => {
    expect(effectiveBpm(120, 0.5, 0.06)).toBeCloseTo(120, 5);
    expect(effectiveBpm(120, 0, 0.06)).toBeCloseTo(127.2, 5);
    expect(effectiveBpm(null, 0.5)).toBeNull();
    expect(effectiveBpm(0, 0.5)).toBeNull();
  });

  it("formats pitch percent and tempo range labels", () => {
    expect(formatPitchPercent(0.5, 0.06)).toBe("+0.00%");
    expect(formatPitchPercent(0, 0.06)).toBe("+6.00%");
    expect(formatTempoRange(0.06)).toBe("±6%");
    expect(formatTempoRange(0.16)).toBe("±16%");
  });

  it("nudges by percent of rate within the active range", () => {
    const nudged = nudgeSpeed(0.5, 3, 0.06);
    expect(normToSpeedRatio(nudged, 0.06)).toBeCloseTo(1.03, 5);
    expect(nudgeSpeed(0, 10, 0.06)).toBe(0);
  });
});
