import {
  animate,
  type MotionValue,
  useAnimationFrame,
  useMotionValue,
} from "motion/react";
import { useCallback, useEffect, useRef } from "react";

export interface SmoothPlayhead {
  motionPos: MotionValue<number>;
  getPosition: () => number;
  beginScrub: (positionSecs: number) => void;
  updateScrub: (positionSecs: number) => void;
  endScrub: () => void;
  isScrubbing: () => boolean;
}

interface UseSmoothPlayheadOptions {
  positionSecs: number;
  playing: boolean;
  speed?: number;
  maxSecs?: number;
}

const SEEK_SNAP_SECS = 0.18;
const DRIFT_CORRECT_SECS = 0.06;

export function useSmoothPlayhead({
  positionSecs,
  playing,
  speed = 1,
  maxSecs,
}: UseSmoothPlayheadOptions): SmoothPlayhead {
  const motionPos = useMotionValue(positionSecs);
  const scrubbingRef = useRef(false);
  const scrubPosRef = useRef<number | null>(null);
  const engineRef = useRef({ pos: positionSecs, at: performance.now() });
  const speedRef = useRef(speed);
  const maxSecsRef = useRef(maxSecs);

  speedRef.current = speed;
  maxSecsRef.current = maxSecs;

  const clamp = useCallback((value: number) => {
    const max = maxSecsRef.current ?? Number.POSITIVE_INFINITY;
    return Math.min(max, Math.max(0, value));
  }, []);

  useEffect(() => {
    engineRef.current = { pos: positionSecs, at: performance.now() };

    if (scrubbingRef.current) {
      return;
    }

    const current = motionPos.get();
    const delta = Math.abs(positionSecs - current);

    if (!playing || delta >= SEEK_SNAP_SECS) {
      if (!playing && delta > 0.01 && delta < SEEK_SNAP_SECS) {
        void animate(motionPos, clamp(positionSecs), {
          duration: 0.08,
          ease: "easeOut",
        });
      } else {
        motionPos.set(clamp(positionSecs));
      }
    }
  }, [clamp, motionPos, playing, positionSecs]);

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
    const rate = speedRef.current;
    const next = clamp(motionPos.get() + dt * rate);
    motionPos.set(next);

    const { pos, at } = engineRef.current;
    const engineEstimate = clamp(
      pos + ((performance.now() - at) / 1000) * rate,
    );
    const error = engineEstimate - motionPos.get();
    if (Math.abs(error) > DRIFT_CORRECT_SECS) {
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
