import { describe, expect, it } from "vitest";
import { windowPercent } from "@/components/waveform/waveform-window-markers-motion";

describe("windowPercent", () => {
  it("maps cue at window center to 50%", () => {
    expect(windowPercent(10_000, 4_000, 10_000)).toBe(50);
  });

  it("maps cue at window start to 0%", () => {
    expect(windowPercent(10_000, 4_000, 8_000)).toBe(0);
  });

  it("returns 0 when visibleMs is non-positive", () => {
    expect(windowPercent(10_000, 0, 10_000)).toBe(0);
  });
});
