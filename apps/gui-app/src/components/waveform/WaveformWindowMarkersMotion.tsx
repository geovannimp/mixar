import { motion, type MotionValue, useTransform } from "motion/react";
import type { DeckActiveLoop, DeckHotCueMarker } from "@/types";

const HOT_CUE_COLORS = [
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#06b6d4",
  "#3b82f6",
  "#a855f7",
  "#ec4899",
] as const;

interface WaveformWindowMarkersMotionProps {
  motionPos: MotionValue<number>;
  visibleMs: number;
  hotCues?: DeckHotCueMarker[];
  activeLoop?: DeckActiveLoop | null;
}

/** Percent within the visible window centered on `centerMs`. */
export function windowPercent(centerMs: number, visibleMs: number, positionMs: number): number {
  if (visibleMs <= 0) {
    return 0;
  }
  const start = centerMs - visibleMs / 2;
  return ((positionMs - start) / visibleMs) * 100;
}

function HotCueMarkerMotion({
  motionPos,
  visibleMs,
  positionMs,
  color,
}: {
  motionPos: MotionValue<number>;
  visibleMs: number;
  positionMs: number;
  color: string;
}) {
  const left = useTransform(
    motionPos,
    (center) => `${windowPercent(center, visibleMs, positionMs)}%`,
  );
  const opacity = useTransform(motionPos, (center) => {
    const pct = windowPercent(center, visibleMs, positionMs);
    return pct < 0 || pct > 100 ? 0 : 1;
  });

  return (
    <motion.div
      className="absolute top-0 h-full w-px"
      style={{ left, opacity, backgroundColor: color }}
    >
      <div
        className="absolute -top-px left-1/2 size-1.5 -translate-x-1/2 rotate-45 border border-black/40"
        style={{ backgroundColor: color }}
      />
    </motion.div>
  );
}

function LoopRegionMotion({
  motionPos,
  visibleMs,
  inMs,
  outMs,
}: {
  motionPos: MotionValue<number>;
  visibleMs: number;
  inMs: number;
  outMs: number;
}) {
  const left = useTransform(motionPos, (center) => `${windowPercent(center, visibleMs, inMs)}%`);
  const width = useTransform(motionPos, (center) => {
    const startPct = windowPercent(center, visibleMs, inMs);
    const endPct = windowPercent(center, visibleMs, outMs);
    return `${Math.max(0, endPct - startPct)}%`;
  });

  return (
    <motion.div
      className="absolute inset-y-0 border-x border-emerald-400/70 bg-emerald-400/18"
      style={{ left, width }}
    />
  );
}

/** Marker overlays driven by MotionValues — no React setState on playhead ticks. */
export function WaveformWindowMarkersMotion({
  motionPos,
  visibleMs,
  hotCues = [],
  activeLoop = null,
}: WaveformWindowMarkersMotionProps) {
  if (visibleMs <= 0) {
    return null;
  }

  const hasLoop = Boolean(activeLoop?.active);
  const hasHotCues = hotCues.length > 0;
  if (!hasLoop && !hasHotCues) {
    return null;
  }

  return (
    <div className="pointer-events-none absolute inset-0 z-10" aria-hidden>
      {hasLoop && activeLoop ? (
        <LoopRegionMotion
          motionPos={motionPos}
          visibleMs={visibleMs}
          inMs={activeLoop.in_ms}
          outMs={activeLoop.out_ms}
        />
      ) : null}

      {hotCues.map((cue) => {
        const color =
          cue.color ?? HOT_CUE_COLORS[cue.slot % HOT_CUE_COLORS.length] ?? HOT_CUE_COLORS[0];
        return (
          <HotCueMarkerMotion
            key={`hotcue-${cue.slot}`}
            motionPos={motionPos}
            visibleMs={visibleMs}
            positionMs={cue.position_ms}
            color={color}
          />
        );
      })}
    </div>
  );
}
