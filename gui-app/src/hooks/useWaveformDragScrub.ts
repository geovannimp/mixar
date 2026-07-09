import { useCallback, useRef, useState } from "react";

export type WaveformScrubMode = "center" | "track";

export interface WaveformDragScrubConfig {
  enabled: boolean;
  mode: WaveformScrubMode;
  spanSecs: number;
  positionSecs: number;
  estimatedPosition?: () => number;
  playing?: boolean;
  maxSecs?: number;
  onSeek: (positionSecs: number) => void;
  seekThrottleMs?: number;
}

export function useWaveformDragScrub({
  enabled,
  mode,
  spanSecs,
  positionSecs,
  estimatedPosition,
  playing = false,
  maxSecs,
  onSeek,
  seekThrottleMs = 32,
}: WaveformDragScrubConfig) {
  const [scrubbing, setScrubbing] = useState(false);
  const [scrubPosition, setScrubPosition] = useState<number | null>(null);
  const scrubbingRef = useRef(false);
  const anchorRef = useRef({ x: 0, position: 0, width: 1 });
  const lastSeekRef = useRef(0);

  const resolveBasePosition = useCallback(() => {
    if (playing && estimatedPosition) {
      return estimatedPosition();
    }
    return positionSecs;
  }, [estimatedPosition, playing, positionSecs]);

  const displayPosition = scrubPosition ?? resolveBasePosition();

  const clampPosition = useCallback(
    (secs: number) => {
      const max = maxSecs ?? Number.POSITIVE_INFINITY;
      return Math.min(max, Math.max(0, secs));
    },
    [maxSecs],
  );

  const positionFromPointer = useCallback(
    (clientX: number) => {
      const { x, position, width } = anchorRef.current;
      const deltaX = clientX - x;
      const deltaSecs = (deltaX / Math.max(width, 1)) * spanSecs;
      if (mode === "center") {
        return clampPosition(position - deltaSecs);
      }
      return clampPosition(position + deltaSecs);
    },
    [clampPosition, mode, spanSecs],
  );

  const emitSeek = useCallback(
    (secs: number) => {
      const now = performance.now();
      if (now - lastSeekRef.current < seekThrottleMs) {
        return;
      }
      lastSeekRef.current = now;
      onSeek(secs);
    },
    [onSeek, seekThrottleMs],
  );

  const endScrub = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!scrubbingRef.current) {
        return;
      }
      scrubbingRef.current = false;
      setScrubbing(false);
      setScrubPosition(null);
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
    },
    [],
  );

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!enabled || spanSecs <= 0) {
        return;
      }
      event.preventDefault();
      const rect = event.currentTarget.getBoundingClientRect();
      const startPosition = resolveBasePosition();
      anchorRef.current = {
        x: event.clientX,
        position: startPosition,
        width: rect.width,
      };
      scrubbingRef.current = true;
      setScrubbing(true);
      setScrubPosition(startPosition);
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [enabled, resolveBasePosition, spanSecs],
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!scrubbingRef.current || !enabled) {
        return;
      }
      const nextPosition = positionFromPointer(event.clientX);
      setScrubPosition(nextPosition);
      emitSeek(nextPosition);
    },
    [emitSeek, enabled, positionFromPointer],
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!scrubbingRef.current) {
        return;
      }
      const nextPosition = positionFromPointer(event.clientX);
      onSeek(nextPosition);
      endScrub(event);
    },
    [endScrub, onSeek, positionFromPointer],
  );

  const handlePointerCancel = endScrub;

  return {
    scrubbing,
    displayPosition,
    handlers: {
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: handlePointerUp,
      onPointerCancel: handlePointerCancel,
    },
    cursorClass: enabled
      ? scrubbing
        ? "cursor-grabbing"
        : "cursor-grab"
      : "",
  };
}
