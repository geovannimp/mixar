import { useCallback, useRef, useState } from "react";
import {
  type SmoothPlayhead,
  useSmoothPlayhead,
} from "./useSmoothPlayhead";

export type WaveformScrubMode = "center" | "track";

export interface WaveformDragScrubConfig {
  enabled: boolean;
  mode: WaveformScrubMode;
  spanSecs: number;
  positionSecs: number;
  playing?: boolean;
  speed?: number;
  maxSecs?: number;
  onSeek: (positionSecs: number) => void;
  seekThrottleMs?: number;
  /** Reuse an existing smooth playhead (DualDeckWaveform + lane renderer). */
  playhead?: SmoothPlayhead;
}

export function useWaveformDragScrub({
  enabled,
  mode,
  spanSecs,
  positionSecs,
  playing = false,
  speed = 1,
  maxSecs,
  onSeek,
  seekThrottleMs = 32,
  playhead: externalPlayhead,
}: WaveformDragScrubConfig) {
  const internalPlayhead = useSmoothPlayhead({
    positionSecs,
    playing,
    speed,
    maxSecs,
  });
  const playhead = externalPlayhead ?? internalPlayhead;

  const [scrubbing, setScrubbing] = useState(false);
  const scrubbingRef = useRef(false);
  const anchorRef = useRef({ x: 0, position: 0, width: 1 });
  const lastSeekRef = useRef(0);

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
      playhead.endScrub();
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
    },
    [playhead],
  );

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!enabled || spanSecs <= 0) {
        return;
      }
      event.preventDefault();
      const rect = event.currentTarget.getBoundingClientRect();
      const startPosition = playhead.getPosition();
      anchorRef.current = {
        x: event.clientX,
        position: startPosition,
        width: rect.width,
      };
      scrubbingRef.current = true;
      setScrubbing(true);
      playhead.beginScrub(startPosition);
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [enabled, playhead, spanSecs],
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!scrubbingRef.current || !enabled) {
        return;
      }
      const nextPosition = positionFromPointer(event.clientX);
      playhead.updateScrub(nextPosition);
      emitSeek(nextPosition);
    },
    [emitSeek, enabled, playhead, positionFromPointer],
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!scrubbingRef.current) {
        return;
      }
      const nextPosition = positionFromPointer(event.clientX);
      playhead.updateScrub(nextPosition);
      onSeek(nextPosition);
      endScrub(event);
    },
    [endScrub, onSeek, playhead, positionFromPointer],
  );

  const handlePointerCancel = endScrub;

  return {
    scrubbing,
    playhead,
    getPosition: playhead.getPosition,
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
