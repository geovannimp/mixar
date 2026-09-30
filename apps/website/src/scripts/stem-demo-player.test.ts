import assert from "node:assert/strict";
import { formatDeckTime, peaksFromBuffer } from "./stem-demo-player.ts";

assert.equal(formatDeckTime(0), "0:00.0");
assert.equal(formatDeckTime(90), "1:30.0");
assert.equal(formatDeckTime(90, true), "-1:30.0");
assert.equal(formatDeckTime(-3), "0:00.0");
// 848 tenths = 84.8s — pass as integer-friendly seconds via 848/10.
assert.equal(formatDeckTime(848 / 10, true), "-1:24.8");

// Minimal AudioBuffer stand-in isn't available in node — peaksFromBuffer needs a real buffer.
// Exercise the empty path with a mock shape:
const empty = {
  getChannelData: () => new Float32Array(0),
} as unknown as AudioBuffer;
assert.equal(peaksFromBuffer(empty, 8).length, 8);
assert.ok([...peaksFromBuffer(empty, 8)].every((v) => v === 0));

console.log("stem-demo-player.test.ts: ok");
