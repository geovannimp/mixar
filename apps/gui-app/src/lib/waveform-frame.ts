import type { WaveformFrame } from "@/types";

/** Must match `WAVEFORM_FRAME_*` in `src-tauri/src/waveform_render.rs`. */
export const WAVEFORM_FRAME_MAGIC = "WFR1";
export const WAVEFORM_FRAME_HEADER_LEN = 28;

/** Normalize Tauri invoke raw/`Response` payloads into a byte view. */
export function toUint8Array(raw: ArrayBuffer | Uint8Array | number[]): Uint8Array {
  if (raw instanceof Uint8Array) {
    return raw;
  }
  if (raw instanceof ArrayBuffer) {
    return new Uint8Array(raw);
  }
  return Uint8Array.from(raw);
}

/** Decode packed `render_waveform_lane` bytes (header + RGBA). */
export function decodeWaveformFrame(raw: ArrayBuffer | Uint8Array | number[]): WaveformFrame {
  const bytes = toUint8Array(raw);
  if (bytes.length < WAVEFORM_FRAME_HEADER_LEN) {
    throw new Error(`waveform frame too short: ${bytes.length}`);
  }

  const magic = String.fromCharCode(bytes[0]!, bytes[1]!, bytes[2]!, bytes[3]!);
  if (magic !== WAVEFORM_FRAME_MAGIC) {
    throw new Error(`waveform frame bad magic: ${magic}`);
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width = view.getUint32(4, true);
  const height = view.getUint32(8, true);
  const center_ms = view.getInt32(12, true);
  const cover_start_ms = view.getInt32(16, true);
  const cover_end_ms = view.getInt32(20, true);
  const visible_ms = view.getInt32(24, true);
  const expected = width * height * 4;
  if (bytes.length < WAVEFORM_FRAME_HEADER_LEN + expected) {
    throw new Error(
      `waveform frame rgba truncated: got ${bytes.length - WAVEFORM_FRAME_HEADER_LEN}, expected ${expected}`,
    );
  }

  // Copy so ImageData owns a tightly-sized ArrayBuffer (required by the canvas API).
  const rgba = new Uint8ClampedArray(expected) as Uint8ClampedArray<ArrayBuffer>;
  rgba.set(bytes.subarray(WAVEFORM_FRAME_HEADER_LEN, WAVEFORM_FRAME_HEADER_LEN + expected));

  return {
    width,
    height,
    rgba,
    center_ms,
    cover_start_ms,
    cover_end_ms,
    visible_ms,
  };
}
