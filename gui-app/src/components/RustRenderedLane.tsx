import { motion, type MotionValue, useMotionValue, useTransform } from "motion/react";
import { useEffect, useRef } from "react";
import type { WaveformTrackCache } from "../lib/waveformTrackCache";

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

export function RustRenderedLane({
  trackCache,
  tileRevision,
  viewportWidth,
  speed = 1,
  motionPos,
  label,
  labelClass,
}: RustRenderedLaneProps) {
  const stripCanvasRef = useRef<HTMLCanvasElement>(null);

  const pxPerSec = trackCache?.pxPerSec ?? 0;
  const stripWidth = trackCache?.canvas.width ?? 0;
  const stripHeight = trackCache?.height ?? 0;
  const safeSpeed = Number.isFinite(speed) && speed > 0 ? Math.min(2, Math.max(0.5, speed)) : 1;
  const displayStripWidth = stripWidth > 0 ? stripWidth / safeSpeed : 0;

  const speedMV = useMotionValue(safeSpeed);
  speedMV.set(safeSpeed);

  const stripX = useTransform([motionPos, speedMV], ([positionSecs, spd]) => {
    const rate = pxPerSec / (typeof spd === "number" && spd > 0 ? spd : 1);
    if (rate <= 0 || viewportWidth <= 0) {
      return 0;
    }
    return viewportWidth / 2 - (positionSecs as number) * rate;
  });

  useEffect(() => {
    const strip = stripCanvasRef.current;
    if (!strip || !trackCache) {
      return;
    }

    if (strip.width !== trackCache.canvas.width) {
      strip.width = trackCache.canvas.width;
    }
    if (strip.height !== trackCache.canvas.height) {
      strip.height = trackCache.canvas.height;
    }

    const ctx = strip.getContext("2d");
    if (!ctx) {
      return;
    }
    ctx.drawImage(trackCache.canvas, 0, 0);
  }, [trackCache, tileRevision]);

  return (
    <div className="absolute inset-0 overflow-hidden bg-black">
      {trackCache && stripWidth > 0 ? (
        <motion.div
          className="absolute top-0 left-0 h-full"
          style={{
            x: stripX,
            width: displayStripWidth,
            height: stripHeight,
          }}
        >
          <canvas
            ref={stripCanvasRef}
            width={stripWidth}
            height={stripHeight}
            className="block h-full w-full"
            style={{ width: displayStripWidth, height: stripHeight }}
            aria-hidden
          />
        </motion.div>
      ) : null}
      <span
        className={`pointer-events-none absolute left-2 top-1 z-10 text-[10px] font-bold uppercase tracking-widest ${labelClass}`}
      >
        {label}
      </span>
    </div>
  );
}

export { useLaneDimensions } from "./useLaneDimensions";
