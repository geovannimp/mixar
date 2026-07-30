import { useCallback, useRef, useState } from "react";
import { type SmoothPlayhead, useSmoothPlayhead } from "./useSmoothPlayhead";

export type WaveformScrubMode = "center" | "track";

export interface WaveformDragScrubConfig {
  enabled: boolean;
  mode: WaveformScrubMode;
  spanMs: number;
  positionMs: number;
  playing?: boolean;
  speed?: number;
  maxMs?: number;
  onSeek: (positionMs: number) => void;
  seekThrottleMs?: number;
  /** Reuse an existing smooth playhead (DualDeckWaveform + lane renderer). */
  playhead?: SmoothPlayhead;
}

export function useWaveformDragScrub({
  enabled,
  mode,
  spanMs,
  positionMs,
  playing = false,
  speed = 1,
  maxMs,
  onSeek,
  seekThrottleMs = 32,
  playhead: externalPlayhead,
}: WaveformDragScrubConfig) {
  const internalPlayhead = useSmoothPlayhead({
    positionMs,
    playing,
    speed,
    maxMs,
  });
  const playhead = externalPlayhead ?? internalPlayhead;

  const [scrubbing, setScrubbing] = useState(false);
  const scrubbingRef = useRef(false);
  const anchorRef = useRef({ x: 0, position: 0, width: 1 });
  const lastSeekRef = useRef(0);

  // Upper bound only when known; do not floor at 0 (negative seek/cue allowed).
  const clampPosition = useCallback(
    (ms: number) => {
      const max = maxMs ?? Number.POSITIVE_INFINITY;
      return Math.min(max, ms);
    },
    [maxMs],
  );

  const positionFromPointer = useCallback(
    (clientX: number) => {
      const { x, position, width } = anchorRef.current;
      const deltaX = clientX - x;
      const deltaMs = (deltaX / Math.max(width, 1)) * spanMs;
      if (mode === "center") {
        return clampPosition(position - deltaMs);
      }
      return clampPosition(position + deltaMs);
    },
    [clampPosition, mode, spanMs],
  );

  const emitSeek = useCallback(
    (ms: number) => {
      const now = performance.now();
      if (now - lastSeekRef.current < seekThrottleMs) {
        return;
      }
      lastSeekRef.current = now;
      onSeek(ms);
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
      if (!enabled || spanMs <= 0) {
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
    [enabled, playhead, spanMs],
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
    cursorClass: enabled ? (scrubbing ? "cursor-grabbing" : "cursor-grab") : "",
  };
}
