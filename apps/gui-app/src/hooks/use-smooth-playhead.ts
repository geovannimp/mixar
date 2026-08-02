import { animate, type MotionValue, useAnimationFrame, useMotionValue } from "motion/react";
import { useCallback, useEffect, useRef } from "react";

export interface SmoothPlayhead {
  motionPos: MotionValue<number>;
  getPosition: () => number;
  beginScrub: (positionMs: number) => void;
  updateScrub: (positionMs: number) => void;
  endScrub: () => void;
  isScrubbing: () => boolean;
}

interface UseSmoothPlayheadOptions {
  positionMs: number;
  playing: boolean;
  speed?: number;
  maxMs?: number;
}

const SEEK_SNAP_MS = 180;
const DRIFT_CORRECT_MS = 60;
/** Soft animate threshold while paused (ms). */
const PAUSE_ANIMATE_MIN_MS = 10;

export function useSmoothPlayhead({
  positionMs,
  playing,
  speed = 1,
  maxMs,
}: UseSmoothPlayheadOptions): SmoothPlayhead {
  const motionPos = useMotionValue(positionMs);
  const scrubbingRef = useRef(false);
  const scrubPosRef = useRef<number | null>(null);
  const engineRef = useRef({ pos: positionMs, at: performance.now() });
  const speedRef = useRef(speed);
  const maxMsRef = useRef(maxMs);

  maxMsRef.current = maxMs;

  // Upper bound only when known; do not floor at 0 (negative seek/cue allowed).
  const clamp = useCallback((value: number) => {
    const max = maxMsRef.current ?? Number.POSITIVE_INFINITY;
    return Math.min(max, value);
  }, []);

  // Rebase the engine anchor when tempo changes so drift correction does not
  // apply the new rate to the whole interval since the last position poll.
  useEffect(() => {
    if (speedRef.current === speed) {
      return;
    }
    speedRef.current = speed;
    if (scrubbingRef.current) {
      return;
    }
    engineRef.current = { pos: motionPos.get(), at: performance.now() };
  }, [motionPos, speed]);

  useEffect(() => {
    engineRef.current = { pos: positionMs, at: performance.now() };

    if (scrubbingRef.current) {
      return;
    }

    const current = motionPos.get();
    const delta = Math.abs(positionMs - current);

    if (!playing || delta >= SEEK_SNAP_MS) {
      if (!playing && delta > PAUSE_ANIMATE_MIN_MS && delta < SEEK_SNAP_MS) {
        void animate(motionPos, clamp(positionMs), {
          duration: 0.08,
          ease: "easeOut",
        });
      } else {
        motionPos.set(clamp(positionMs));
      }
    }
  }, [clamp, motionPos, playing, positionMs]);

  useAnimationFrame((_, deltaMs) => {
    if (scrubbingRef.current) {
      const scrubPos = scrubPosRef.current;
      if (scrubPos != null) {
        motionPos.set(clamp(scrubPos));
      }
      return;
    }

    if (!playing) {
      return;
    }

    const dt = deltaMs / 1000;
    const rateMs = speedRef.current * 1000;
    const next = clamp(motionPos.get() + dt * rateMs);
    motionPos.set(next);

    const { pos, at } = engineRef.current;
    const engineEstimate = clamp(pos + ((performance.now() - at) / 1000) * rateMs);
    const error = engineEstimate - motionPos.get();
    if (Math.abs(error) > DRIFT_CORRECT_MS) {
      motionPos.set(clamp(motionPos.get() + error * 0.25));
    }
  });

  const beginScrub = useCallback(
    (start: number) => {
      scrubbingRef.current = true;
      scrubPosRef.current = start;
      motionPos.set(clamp(start));
    },
    [clamp, motionPos],
  );

  const updateScrub = useCallback(
    (next: number) => {
      scrubPosRef.current = next;
      motionPos.set(clamp(next));
    },
    [clamp, motionPos],
  );

  const endScrub = useCallback(() => {
    scrubbingRef.current = false;
    scrubPosRef.current = null;
  }, []);

  const getPosition = useCallback(() => motionPos.get(), [motionPos]);

  return {
    motionPos,
    getPosition,
    beginScrub,
    updateScrub,
    endScrub,
    isScrubbing: () => scrubbingRef.current,
  };
}
