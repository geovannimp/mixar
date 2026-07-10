import {
  type MotionValue,
  useAnimationFrame,
  useMotionValue,
} from "motion/react";
import { useCallback, useEffect, useRef } from "react";

interface UseSmoothTrackProgressOptions {
  positionSecs: number;
  durationSecs: number | null | undefined;
  playing: boolean;
  speed?: number;
}

/** Normalized 0–1 playhead progress; snaps on seek/cue, smooth during playback. */
export function useSmoothTrackProgress({
  positionSecs,
  durationSecs,
  playing,
  speed = 1,
}: UseSmoothTrackProgressOptions): MotionValue<number> {
  const duration =
    durationSecs != null && durationSecs > 0 ? durationSecs : 0;

  const toProgress = useCallback(
    (secs: number) => {
      if (duration <= 0) {
        return 0;
      }
      return Math.min(1, Math.max(0, secs / duration));
    },
    [duration],
  );

  const motionProgress = useMotionValue(toProgress(positionSecs));
  const engineRef = useRef({ pos: positionSecs, at: performance.now() });
  const speedRef = useRef(speed);

  speedRef.current = speed;

  useEffect(() => {
    engineRef.current = { pos: positionSecs, at: performance.now() };
    const target = toProgress(positionSecs);
    const current = motionProgress.get();
    const delta = Math.abs(target - current);

    // Snap on seek/cue; small drift during playback is handled in the rAF loop.
    if (delta > 0.015 || !playing) {
      motionProgress.set(target);
    }
  }, [motionProgress, playing, positionSecs, toProgress]);

  useAnimationFrame((_, deltaMs) => {
    if (!playing || duration <= 0) {
      return;
    }

    const dt = deltaMs / 1000;
    const rate = speedRef.current;
    const next = Math.min(
      1,
      motionProgress.get() + (dt / duration) * rate,
    );
    motionProgress.set(next);

    const { pos, at } = engineRef.current;
    const engineEstimate = toProgress(
      pos + ((performance.now() - at) / 1000) * rate,
    );
    const error = engineEstimate - motionProgress.get();
    if (Math.abs(error) > 0.02) {
      motionProgress.set(
        Math.min(1, Math.max(0, motionProgress.get() + error * 0.25)),
      );
    }
  });

  return motionProgress;
}
