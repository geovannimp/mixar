import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import type { DeckEq, WaveformFrame } from "../types";
import {
  WAVEFORM_BUFFER_RATIO,
  WAVEFORM_REFRESH_MARGIN,
  WAVEFORM_VISIBLE_SECS,
} from "../lib/spectralColor";

interface UseRenderWaveformLaneOptions {
  trackId: string | null;
  path: string | null;
  positionSecs: number;
  playing: boolean;
  speed?: number;
  eq: DeckEq;
  width: number;
  height: number;
  getPosition: () => number;
  isScrubbing?: () => boolean;
}

export function useRenderWaveformLane({
  trackId,
  path,
  positionSecs,
  playing,
  speed = 1,
  eq,
  width,
  height,
  getPosition,
  isScrubbing,
}: UseRenderWaveformLaneOptions) {
  const [frame, setFrame] = useState<WaveformFrame | null>(null);
  const [loading, setLoading] = useState(false);

  const frameRef = useRef<WaveformFrame | null>(null);
  const inFlightRef = useRef(false);
  const requestIdRef = useRef(0);

  const enginePosRef = useRef(positionSecs);
  const engineAtRef = useRef(performance.now());
  const playingRef = useRef(playing);
  const speedRef = useRef(speed);
  const getPositionRef = useRef(getPosition);
  const isScrubbingRef = useRef(isScrubbing);

  playingRef.current = playing;
  speedRef.current = speed;
  getPositionRef.current = getPosition;
  isScrubbingRef.current = isScrubbing;

  useEffect(() => {
    enginePosRef.current = positionSecs;
    engineAtRef.current = performance.now();
  }, [positionSecs]);

  const estimatedPosition = useCallback(() => {
    if (isScrubbingRef.current?.()) {
      return getPositionRef.current();
    }
    if (!playingRef.current) {
      return enginePosRef.current;
    }
    const elapsed = (performance.now() - engineAtRef.current) / 1000;
    return enginePosRef.current + elapsed * speedRef.current;
  }, []);

  const fetchStrip = useCallback(
    async (position: number, includeDetail: boolean) => {
      if ((!trackId && !path) || width <= 0 || height <= 0) {
        frameRef.current = null;
        setFrame(null);
        return;
      }

      if (inFlightRef.current) {
        return;
      }

      inFlightRef.current = true;
      const requestId = ++requestIdRef.current;
      setLoading(true);

      try {
        const nextFrame = await invoke<WaveformFrame>("render_waveform_lane", {
          trackId,
          path: trackId ? null : path,
          width,
          height,
          positionSecs: position,
          visibleSecs: WAVEFORM_VISIBLE_SECS,
          bufferRatio: WAVEFORM_BUFFER_RATIO,
          includeDetail,
          includeBeatGrid: true,
          eqLowDb: eq.low,
          eqMidDb: eq.mid,
          eqHighDb: eq.high,
        });

        if (requestId === requestIdRef.current) {
          frameRef.current = nextFrame;
          setFrame(nextFrame);
        }
      } catch (err) {
        console.error("render_waveform_lane failed", err);
        if (requestId === requestIdRef.current) {
          frameRef.current = null;
          setFrame(null);
        }
      } finally {
        inFlightRef.current = false;
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    },
    [trackId, path, width, height, eq.low, eq.mid, eq.high],
  );

  useEffect(() => {
    frameRef.current = null;
    setFrame(null);
    const pos = getPositionRef.current();
    void fetchStrip(pos, false).then(() => fetchStrip(pos, true));
  }, [trackId, path, width, height, fetchStrip]);

  useEffect(() => {
    if (playing || isScrubbingRef.current?.()) {
      return;
    }
    const current = frameRef.current;
    if (!current || needsRefresh(current, positionSecs)) {
      void fetchStrip(positionSecs, true);
    }
  }, [playing, positionSecs, fetchStrip]);

  useEffect(() => {
    if (!playing) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      if (isScrubbingRef.current?.()) {
        frameId = window.requestAnimationFrame(tick);
        return;
      }

      const estimated = getPositionRef.current();
      const current = frameRef.current;
      if (current && needsRefresh(current, estimated) && !inFlightRef.current) {
        void fetchStrip(estimated, true);
      }
      frameId = window.requestAnimationFrame(tick);
    };
    frameId = window.requestAnimationFrame(tick);

    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [playing, fetchStrip]);

  return { frame, estimatedPosition, loading };
}

function needsRefresh(frame: WaveformFrame, positionSecs: number): boolean {
  const halfVisible = frame.visible_secs / 2;
  const leftSlack = positionSecs - halfVisible - frame.cover_start_secs;
  const rightSlack = frame.cover_end_secs - (positionSecs + halfVisible);
  const minSlack =
    frame.visible_secs * WAVEFORM_BUFFER_RATIO * WAVEFORM_REFRESH_MARGIN;
  return leftSlack < minSlack || rightSlack < minSlack;
}
