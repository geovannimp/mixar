import { describe, expect, it } from "vitest";
import { formatLogRecordForSink } from "@/lib/logging";
import type { LogRecord } from "@logtape/logtape";

describe("formatLogRecordForSink", () => {
  it("joins category, message, and properties", () => {
    const record: LogRecord = {
      category: ["gui", "engine"],
      level: "error",
      message: ["engine bus event decode failed"],
      rawMessage: "engine bus event decode failed",
      timestamp: 0,
      properties: { err: "boom" },
    };
    expect(formatLogRecordForSink(record)).toBe(
      '[gui.engine] engine bus event decode failed {"err":"boom"}',
    );
  });
});
