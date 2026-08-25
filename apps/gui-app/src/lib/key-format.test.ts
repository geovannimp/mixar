import { describe, expect, it } from "vitest";
import { formatDeckKey } from "./key-format";

describe("formatDeckKey", () => {
  it("maps Mixed In Key Camelot ↔ musical", () => {
    expect(formatDeckKey("C", "camelot")).toBe("8B");
    expect(formatDeckKey("Am", "camelot")).toBe("8A");
    expect(formatDeckKey("8B", "musical")).toBe("C");
    expect(formatDeckKey("8A", "musical")).toBe("Am");
  });

  it("rejects malformed Camelot number text", () => {
    expect(formatDeckKey("8junkB", "musical")).toBe("8junkB");
    expect(formatDeckKey("8.5B", "musical")).toBe("8.5B");
    expect(formatDeckKey("08B", "musical")).toBe("08B");
    expect(formatDeckKey("+8B", "musical")).toBe("+8B");
    expect(formatDeckKey("13B", "musical")).toBe("13B");
  });
});
