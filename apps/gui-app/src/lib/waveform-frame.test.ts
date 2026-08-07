import { describe, expect, it } from "vitest";
import {
  decodeWaveformFrame,
  WAVEFORM_FRAME_HEADER_LEN,
  WAVEFORM_FRAME_MAGIC,
} from "@/lib/waveform-frame";

function packFrame(opts: {
  width: number;
  height: number;
  centerMs?: number;
  coverStartMs?: number;
  coverEndMs?: number;
  visibleMs?: number;
  rgba: Uint8Array;
}): Uint8Array {
  const {
    width,
    height,
    centerMs = 0,
    coverStartMs = 0,
    coverEndMs = 0,
    visibleMs = 0,
    rgba,
  } = opts;
  const out = new Uint8Array(WAVEFORM_FRAME_HEADER_LEN + rgba.length);
  const view = new DataView(out.buffer);
  for (let i = 0; i < 4; i += 1) {
    out[i] = WAVEFORM_FRAME_MAGIC.charCodeAt(i);
  }
  view.setUint32(4, width, true);
  view.setUint32(8, height, true);
  view.setInt32(12, centerMs, true);
  view.setInt32(16, coverStartMs, true);
  view.setInt32(20, coverEndMs, true);
  view.setInt32(24, visibleMs, true);
  out.set(rgba, WAVEFORM_FRAME_HEADER_LEN);
  return out;
}

describe("decodeWaveformFrame", () => {
  it("parses header and rgba from packed bytes", () => {
    const rgba = new Uint8Array([10, 20, 30, 255, 40, 50, 60, 255]);
    const packed = packFrame({
      width: 2,
      height: 1,
      centerMs: 1500,
      coverStartMs: 0,
      coverEndMs: 3000,
      visibleMs: 3000,
      rgba,
    });

    const frame = decodeWaveformFrame(packed);
    expect(frame.width).toBe(2);
    expect(frame.height).toBe(1);
    expect(frame.center_ms).toBe(1500);
    expect(frame.cover_start_ms).toBe(0);
    expect(frame.cover_end_ms).toBe(3000);
    expect(frame.visible_ms).toBe(3000);
    expect(Array.from(frame.rgba)).toEqual(Array.from(rgba));
  });

  it("accepts ArrayBuffer and number[] payloads", () => {
    const rgba = new Uint8Array([1, 2, 3, 255]);
    const packed = packFrame({ width: 1, height: 1, rgba });
    expect(decodeWaveformFrame(packed.buffer as ArrayBuffer).rgba[0]).toBe(1);
    expect(decodeWaveformFrame(Array.from(packed)).rgba[3]).toBe(255);
  });
});
