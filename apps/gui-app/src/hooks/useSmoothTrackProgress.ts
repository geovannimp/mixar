import { type MotionValue, useAnimationFrame, useMotionValue } from "motion/react";
import { useCallback, useEffect, useRef } from "react";

interface UseSmoothTrackProgressOptions {
  positionMs: number;
  durationMs: number | null | undefined;
  playing: boolean;
  speed?: number;
}

/** Normalized 0–1 playhead progress; snaps on seek/cue, smooth during playback. */
export function useSmoothTrackProgress({
  positionMs,
  durationMs,
  playing,
  speed = 1,
}: UseSmoothTrackProgressOptions): MotionValue<number> {
  const duration = durationMs != null && durationMs > 0 ? durationMs : 0;

  const toProgress = useCallback(
    (ms: number) => {
      if (duration <= 0) {
        return 0;
      }
      return Math.min(1, Math.max(0, ms / duration));
    },
    [duration],
  );

  const motionProgress = useMotionValue(toProgress(positionMs));
  const engineRef = useRef({ pos: positionMs, at: performance.now() });
  const speedRef = useRef(speed);

  // Rebase when tempo changes so the new rate only applies going forward.
  useEffect(() => {
    if (speedRef.current === speed) {
      return;
    }
    speedRef.current = speed;
    if (duration <= 0) {
      return;
    }
    engineRef.current = {
      pos: motionProgress.get() * duration,
      at: performance.now(),
    };
  }, [duration, motionProgress, speed]);

  useEffect(() => {
    engineRef.current = { pos: positionMs, at: performance.now() };
    const target = toProgress(positionMs);
    const current = motionProgress.get();
    const delta = Math.abs(target - current);

    // Snap on seek/cue; small drift during playback is handled in the rAF loop.
    if (delta > 0.015 || !playing) {
      motionProgress.set(target);
    }
  }, [motionProgress, playing, positionMs, toProgress]);

  useAnimationFrame((_, deltaMs) => {
    if (!playing || duration <= 0) {
      return;
    }

    const dt = deltaMs / 1000;
    const rateMs = speedRef.current * 1000;
    const next = Math.min(1, motionProgress.get() + (dt * rateMs) / duration);
    motionProgress.set(next);

    const { pos, at } = engineRef.current;
    const engineEstimate = toProgress(pos + ((performance.now() - at) / 1000) * rateMs);
    const error = engineEstimate - motionProgress.get();
    if (Math.abs(error) > 0.02) {
      motionProgress.set(Math.min(1, Math.max(0, motionProgress.get() + error * 0.25)));
    }
  });

  return motionProgress;
}
