import { describe, expect, it } from "vitest";
import { formatBeatLength } from "./pad-modes";

describe("formatBeatLength", () => {
  it("formats fractions below one beat", () => {
    expect(formatBeatLength(1 / 32)).toBe("1/32");
    expect(formatBeatLength(0.5)).toBe("1/2");
  });

  it("stringifies whole and non-positive values", () => {
    expect(formatBeatLength(1)).toBe("1");
    expect(formatBeatLength(4)).toBe("4");
    expect(formatBeatLength(0)).toBe("0");
    expect(formatBeatLength(-2)).toBe("-2");
  });
});
