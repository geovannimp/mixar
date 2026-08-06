import { describe, expect, it } from "vitest";
import type { LogRecord } from "@logtape/logtape";
import { formatLogRecordForSink } from "@/lib/logging";

describe("formatLogRecordForSink", () => {
  it("joins category, message, and properties", () => {
    const record: LogRecord = {
      category: ["app", "engine"],
      level: "error",
      message: ["engine bus event decode failed"],
      rawMessage: "engine bus event decode failed",
      timestamp: 0,
      properties: { err: "boom" },
    };
    expect(formatLogRecordForSink(record)).toBe(
      '[app.engine] engine bus event decode failed {"err":"boom"}',
    );
  });

  it("serializes Error properties", () => {
    const err = new Error("boom");
    const record: LogRecord = {
      category: ["app"],
      level: "error",
      message: ["failed"],
      rawMessage: "failed",
      timestamp: 0,
      properties: { err },
    };
    expect(formatLogRecordForSink(record)).toContain('"message":"boom"');
  });
});
