import { type MotionValue, useMotionValueEvent } from "motion/react";
import { useEffect, useRef } from "react";
import type { WaveformTrackCache } from "@/lib/waveform-track-cache";

/** How many viewport-widths of waveform to keep in the sliding DOM canvas. */
const BUFFER_VIEWPORTS = 3;

interface RustRenderedLaneProps {
  trackCache: WaveformTrackCache | null;
  tileRevision: number;
  viewportWidth: number;
  /** Playback speed — scales horizontal zoom so beat spacing tracks effective BPM. */
  speed?: number;
  motionPos: MotionValue<number>;
  label: string;
  labelClass: string;
}

/**
 * Sliding-window waveform lane.
 *
 * The full-track strip stays off-DOM (up to ~16k px). We copy a ~3×-viewport
 * window into a small canvas and translate it with CSS on playhead ticks.
 * Sampling the giant strip every frame was ~300ms/frame in WebView; transform
 * of a small layer is cheap.
 */
export function RustRenderedLane({
  trackCache,
  tileRevision,
  viewportWidth,
  speed = 1,
  motionPos,
  label,
  labelClass,
}: RustRenderedLaneProps) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const trackCacheRef = useRef(trackCache);
  const viewportWidthRef = useRef(viewportWidth);
  const speedRef = useRef(1);
  const originMsRef = useRef(0);
  const bufferMsRef = useRef(0);

  const safeSpeed = Number.isFinite(speed) && speed > 0 ? Math.min(2, Math.max(0.5, speed)) : 1;
  trackCacheRef.current = trackCache;
  viewportWidthRef.current = viewportWidth;
  speedRef.current = safeSpeed;

  const applyTransform = (positionMs: number) => {
    const wrap = wrapRef.current;
    const cache = trackCacheRef.current;
    const width = viewportWidthRef.current;
    if (!wrap || !cache || width <= 0 || cache.pxPerMs <= 0) {
      return;
    }
    const rate = cache.pxPerMs / speedRef.current;
    const x = width / 2 - (positionMs - originMsRef.current) * rate;
    wrap.style.transform = `translate3d(${x}px,0,0)`;
  };

  const rebuildBuffer = (positionMs: number) => {
    const canvas = canvasRef.current;
    const cache = trackCacheRef.current;
    const width = viewportWidthRef.current;
    if (!canvas || !cache || width <= 0 || cache.pxPerMs <= 0) {
      return;
    }

    const spd = speedRef.current;
    const rate = cache.pxPerMs / spd;
    const height = cache.height;
    const bufW = Math.max(1, Math.round(width * BUFFER_VIEWPORTS));
    const bufMs = bufW / rate;
    const originMs = positionMs - bufMs / 2;

    originMsRef.current = originMs;
    bufferMsRef.current = bufMs;

    if (canvas.width !== bufW) {
      canvas.width = bufW;
    }
    if (canvas.height !== height) {
      canvas.height = height;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    // Strip pixels → buffer display pixels (speed zooms the window).
    const sx = originMs * cache.pxPerMs;
    const srcW = bufMs * cache.pxPerMs;

    ctx.fillStyle = "#050508";
    ctx.fillRect(0, 0, bufW, height);
    ctx.drawImage(cache.canvas, sx, 0, srcW, height, 0, 0, bufW, height);

    const wrap = wrapRef.current;
    if (wrap) {
      wrap.style.width = `${bufW}px`;
      wrap.style.height = `${height}px`;
    }
    applyTransform(positionMs);
  };

  const syncToPlayhead = (positionMs: number) => {
    const width = viewportWidthRef.current;
    const bufMs = bufferMsRef.current;
    if (width <= 0 || bufMs <= 0) {
      rebuildBuffer(positionMs);
      return;
    }

    // Rebuild when the playhead leaves the middle third of the buffer.
    const origin = originMsRef.current;
    const visibleMs = bufMs / BUFFER_VIEWPORTS;
    const low = origin + visibleMs;
    const high = origin + bufMs - visibleMs;
    if (positionMs < low || positionMs > high) {
      rebuildBuffer(positionMs);
      return;
    }
    applyTransform(positionMs);
  };

  useMotionValueEvent(motionPos, "change", syncToPlayhead);

  useEffect(() => {
    rebuildBuffer(motionPos.get());
  }, [trackCache, tileRevision, viewportWidth, safeSpeed, motionPos]);

  return (
    <div className="absolute inset-0 overflow-hidden bg-black">
      {trackCache && viewportWidth > 0 ? (
        <div ref={wrapRef} className="absolute top-0 left-0 will-change-transform">
          <canvas ref={canvasRef} className="block h-full w-full" aria-hidden />
        </div>
      ) : null}
      <span
        className={`pointer-events-none absolute left-2 top-1 z-10 text-[10px] font-bold uppercase tracking-widest ${labelClass}`}
      >
        {label}
      </span>
    </div>
  );
}
