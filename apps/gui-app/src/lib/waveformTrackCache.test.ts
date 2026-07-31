import { describe, expect, it } from "vitest";
import { computePxPerMs, MAX_WAVEFORM_CANVAS_WIDTH } from "@/lib/waveformTrackCache";

describe("computePxPerMs", () => {
  it("keeps ideal density for short tracks", () => {
    const px = computePxPerMs(1200, 181_000, 24_000);
    expect(px).toBeCloseTo(1200 / 24_000);
    expect(181_000 * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH);
  });

  it("caps density so long tracks stay within canvas width", () => {
    const duration = 387_000;
    const px = computePxPerMs(1200, duration, 24_000);
    expect(duration * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH + 1e-6);
    expect(px).toBeLessThan(1200 / 24_000);
  });
});
