import { describe, expect, it } from "vitest";
import { degreesToJogTicks, JOG_INTERVALS_PER_REV } from "./jogTicks";

describe("degreesToJogTicks", () => {
  it("maps a full turn to intervals_per_rev", () => {
    expect(degreesToJogTicks(360)).toBe(JOG_INTERVALS_PER_REV);
  });

  it("rounds partial turns", () => {
    expect(degreesToJogTicks(180)).toBe(JOG_INTERVALS_PER_REV / 2);
  });

  it("returns 0 for empty input", () => {
    expect(degreesToJogTicks(0)).toBe(0);
    expect(degreesToJogTicks(Number.NaN)).toBe(0);
  });
});
