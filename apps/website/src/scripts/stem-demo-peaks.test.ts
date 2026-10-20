import { describe, expect, it } from "vitest";

import { barHeightsFromPeaks } from "./stem-demo-player";

describe("barHeightsFromPeaks", () => {
  it("keeps the max of each downsampling window", () => {
    // 8 peaks -> 4 bars, each bar spans two peaks.
    const peaks = new Float32Array([0.1, 0.5, 0.2, 0.9, 0.3, 0.4, 0.8, 0.2]);
    expect(barHeightsFromPeaks(peaks, 4)).toEqual(["52.5%", "90.5%", "43%", "81%"]);
  });

  it("maps peaks one-to-one when counts match", () => {
    expect(barHeightsFromPeaks(new Float32Array([0, 0.5, 1]), 3)).toEqual([
      "5%",
      "52.5%",
      "100%",
    ]);
  });

  it("repeats each peak when there are fewer peaks than bars", () => {
    expect(barHeightsFromPeaks(new Float32Array([0.2, 0.9]), 4)).toEqual([
      "24%",
      "24%",
      "90.5%",
      "90.5%",
    ]);
  });

  it("clamps samples above 1 so bars cannot overflow the lane", () => {
    expect(barHeightsFromPeaks(new Float32Array([1.4]), 1)).toEqual(["100%"]);
  });

  it("falls back to the skeleton height when no peaks are available", () => {
    expect(barHeightsFromPeaks(new Float32Array(0), 3)).toEqual(["5%", "5%", "5%"]);
  });

  it("returns no bars when the bar count is not positive", () => {
    expect(barHeightsFromPeaks(new Float32Array([0.5]), 0)).toEqual([]);
  });
});
