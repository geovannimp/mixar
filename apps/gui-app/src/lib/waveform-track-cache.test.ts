import { beforeAll, describe, expect, it } from "vitest";
import {
  computePxPerMs,
  MAX_WAVEFORM_CANVAS_WIDTH,
  WaveformTrackCache,
} from "@/lib/waveform-track-cache";
import type { WaveformFrame } from "@/types";

describe("computePxPerMs", () => {
  it("keeps ideal density for short tracks", () => {
    const px = computePxPerMs(1200, 181_000, 24_000);
    expect(px).toBeCloseTo(1200 / 24_000);
    expect(181_000 * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH);
  });

  it("caps density so long tracks stay within canvas width", () => {
    const duration = 387_000;
    const px = computePxPerMs(1200, duration, 24_000);
    expect(duration * px).toBeLessThanOrEqual(MAX_WAVEFORM_CANVAS_WIDTH + 1e-6);
    expect(px).toBeLessThan(1200 / 24_000);
  });
});

describe("WaveformTrackCache tile quality", () => {
  beforeAll(() => {
    class FakeCtx {
      fillStyle = "";
      fillRect() {}
      putImageData() {}
    }
    class FakeCanvas {
      width = 0;
      height = 0;
      getContext() {
        return new FakeCtx();
      }
    }
    // vitest environment is node — stub the canvas bits blit/create need.
    Object.assign(globalThis, {
      document: {
        createElement: () => new FakeCanvas(),
      },
      ImageData: class ImageData {
        data: Uint8ClampedArray;
        width: number;
        height: number;
        constructor(data: Uint8ClampedArray, width: number, height: number) {
          this.data = data;
          this.width = width;
          this.height = height;
        }
      },
    });
  });

  function solidFrame(width = 1, height = 1): WaveformFrame {
    const rgba = new Uint8ClampedArray(width * height * 4) as Uint8ClampedArray<ArrayBuffer>;
    for (let i = 0; i < rgba.length; i += 4) {
      rgba[i] = 10;
      rgba[i + 1] = 10;
      rgba[i + 2] = 10;
      rgba[i + 3] = 255;
    }
    return {
      width,
      height,
      rgba,
      center_ms: 0,
      cover_start_ms: 0,
      cover_end_ms: 1000,
      visible_ms: 1000,
    };
  }

  it("reports missing overview tiles, then allows detail upgrade", () => {
    // 72s track → multiple tiles at 24s visible
    const cache = WaveformTrackCache.create(800, 40, 72_000, 24_000);
    const missingOverview = cache.missingTileIndices(0, 24_000, 0, "overview");
    expect(missingOverview.length).toBeGreaterThan(0);

    const tile = missingOverview[0]!;
    expect(cache.tryMarkPending(tile, "overview")).toBe(true);
    expect(cache.tryMarkPending(tile, "overview")).toBe(false);

    cache.blitTile(solidFrame(), tile, "overview");
    expect(cache.qualityOf(tile)).toBe("overview");
    expect(cache.missingTileIndices(0, 24_000, 0, "overview")).not.toContain(tile);
    expect(cache.missingTileIndices(0, 24_000, 0, "detail")).toContain(tile);

    expect(cache.tryMarkPending(tile, "detail")).toBe(true);
    cache.blitTile(solidFrame(), tile, "detail");
    expect(cache.qualityOf(tile)).toBe("detail");
    expect(cache.missingTileIndices(0, 24_000, 0, "detail")).not.toContain(tile);
    expect(cache.tryMarkPending(tile, "detail")).toBe(false);
    expect(cache.tryMarkPending(tile, "overview")).toBe(false);
  });

  it("ignores stale overview blit after detail", () => {
    const cache = WaveformTrackCache.create(800, 40, 24_000, 24_000);
    expect(cache.tryMarkPending(0, "overview")).toBe(true);
    cache.blitTile(solidFrame(), 0, "overview");
    expect(cache.tryMarkPending(0, "detail")).toBe(true);
    cache.blitTile(solidFrame(), 0, "detail");

    // Late overview must not downgrade quality.
    cache.blitTile(solidFrame(), 0, "overview");
    expect(cache.qualityOf(0)).toBe("detail");
  });
});
