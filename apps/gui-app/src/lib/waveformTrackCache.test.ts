import { describe, expect, it } from "vitest";
import { computePxPerSec, MAX_WAVEFORM_CANVAS_WIDTH } from "@/lib/waveformTrackCache";

describe("computePxPerSec", () => {
  it("keeps ideal density for short tracks", () => {
    const px = computePxPerSec(1200, 181, 24);
    expect(px).toBeCloseTo(1200 / 24);
    expect(181 * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH);
  });

  it("caps density so long tracks stay within canvas width", () => {
    const duration = 387;
    const px = computePxPerSec(1200, duration, 24);
    expect(duration * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH + 1e-6);
    expect(px).toBeLessThan(1200 / 24);
  });
});
