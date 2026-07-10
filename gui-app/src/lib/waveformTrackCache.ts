import type { WaveformFrame } from "../types";
import { WAVEFORM_VISIBLE_SECS } from "./spectralColor";

const MIN_TILES = 1;
const MAX_TILES = 48;

function decodeBase64Rgba(base64: string): Uint8ClampedArray {
  const binary = atob(base64);
  const bytes = new Uint8ClampedArray(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/** Pick tile duration from track length (not a fixed window size). */
export function computeTileSecs(
  durationSecs: number,
  visibleSecs: number = WAVEFORM_VISIBLE_SECS,
): number {
  if (durationSecs <= visibleSecs) {
    return durationSecs;
  }
  const targetTiles = Math.min(
    MAX_TILES,
    Math.max(MIN_TILES, Math.ceil(durationSecs / visibleSecs)),
  );
  return durationSecs / targetTiles;
}

/** Full-track strip filled incrementally; never re-fetches completed tiles. */
export class WaveformTrackCache {
  readonly canvas: HTMLCanvasElement;
  readonly visibleSecs: number;
  readonly tileSecs: number;
  readonly pxPerSec: number;
  readonly durationSecs: number;
  readonly height: number;

  private readonly filledTiles = new Set<number>();
  private readonly pendingTiles = new Set<number>();
  private readonly tileCount: number;
  private revision = 0;

  private constructor(
    canvas: HTMLCanvasElement,
    durationSecs: number,
    visibleSecs: number,
    tileSecs: number,
    pxPerSec: number,
    height: number,
  ) {
    this.canvas = canvas;
    this.durationSecs = durationSecs;
    this.visibleSecs = visibleSecs;
    this.tileSecs = tileSecs;
    this.pxPerSec = pxPerSec;
    this.height = height;
    this.tileCount = Math.max(1, Math.ceil(durationSecs / tileSecs));
  }

  static create(
    viewportWidth: number,
    height: number,
    durationSecs: number,
    visibleSecs: number = WAVEFORM_VISIBLE_SECS,
  ): WaveformTrackCache {
    const safeDuration = Math.max(durationSecs, visibleSecs);
    const tileSecs = computeTileSecs(safeDuration, visibleSecs);
    const pxPerSec = viewportWidth / visibleSecs;
    const canvasWidth = Math.max(1, Math.ceil(safeDuration * pxPerSec));

    const canvas = document.createElement("canvas");
    canvas.width = canvasWidth;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (ctx) {
      ctx.fillStyle = "#050508";
      ctx.fillRect(0, 0, canvasWidth, height);
    }

    return new WaveformTrackCache(
      canvas,
      safeDuration,
      visibleSecs,
      tileSecs,
      pxPerSec,
      height,
    );
  }

  get tileRevision(): number {
    return this.revision;
  }

  tileRange(index: number): { start: number; end: number; duration: number } {
    const start = index * this.tileSecs;
    const end = Math.min(this.durationSecs, start + this.tileSecs);
    return { start, end, duration: Math.max(end - start, 0.1) };
  }

  tileWidthPx(index: number): number {
    const { duration } = this.tileRange(index);
    return Math.max(1, Math.round(duration * this.pxPerSec));
  }

  tryMarkPending(index: number): boolean {
    if (
      index < 0 ||
      index >= this.tileCount ||
      this.filledTiles.has(index) ||
      this.pendingTiles.has(index)
    ) {
      return false;
    }
    this.pendingTiles.add(index);
    return true;
  }

  clearPending(index: number): void {
    this.pendingTiles.delete(index);
  }

  missingTileIndices(
    viewStart: number,
    viewEnd: number,
    prefetchMargin = 1,
  ): number[] {
    const first = Math.floor(viewStart / this.tileSecs) - prefetchMargin;
    const last = Math.floor(viewEnd / this.tileSecs) + prefetchMargin;
    const center = (viewStart + viewEnd) / 2;

    const indices: number[] = [];
    for (
      let index = Math.max(0, first);
      index <= Math.min(this.tileCount - 1, last);
      index += 1
    ) {
      if (!this.filledTiles.has(index) && !this.pendingTiles.has(index)) {
        indices.push(index);
      }
    }

    indices.sort((a, b) => {
      const centerA = (a + 0.5) * this.tileSecs;
      const centerB = (b + 0.5) * this.tileSecs;
      return Math.abs(centerA - center) - Math.abs(centerB - center);
    });

    return indices;
  }

  blitTile(frame: WaveformFrame, tileIndex: number): void {
    const ctx = this.canvas.getContext("2d");
    if (!ctx || frame.width <= 0 || frame.height <= 0) {
      return;
    }

    try {
      const rgba = decodeBase64Rgba(frame.rgba_base64);
      const expected = frame.width * frame.height * 4;
      if (rgba.length !== expected) {
        console.error(
          `waveform tile rgba size mismatch: got ${rgba.length}, expected ${expected}`,
        );
        return;
      }

      const image = new ImageData(rgba, frame.width, frame.height);
      const tile = document.createElement("canvas");
      tile.width = frame.width;
      tile.height = frame.height;
      tile.getContext("2d")?.putImageData(image, 0, 0);

      const { start } = this.tileRange(tileIndex);
      const destX = start * this.pxPerSec;
      ctx.drawImage(tile, destX, 0);

      if (tileIndex >= 0 && tileIndex < this.tileCount) {
        this.filledTiles.add(tileIndex);
        this.revision += 1;
      }
    } catch (err) {
      console.error("failed to blit waveform tile", err);
    } finally {
      this.pendingTiles.delete(tileIndex);
    }
  }
}
