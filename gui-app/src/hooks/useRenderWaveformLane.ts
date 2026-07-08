import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import type { DeckEq, WaveformFrame } from "../types";
import { WAVEFORM_VISIBLE_SECS } from "../lib/spectralColor";

const BUFFER_RATIO = 0.5;
const PLAYING_REFRESH_MS = 80;

interface UseRenderWaveformLaneOptions {
  trackId: string | null;
  path: string | null;
  positionSecs: number;
  playing: boolean;
  eq: DeckEq;
  width: number;
  height: number;
}

export function useRenderWaveformLane({
  trackId,
  path,
  positionSecs,
  playing,
  eq,
  width,
  height,
}: UseRenderWaveformLaneOptions) {
  const [frame, setFrame] = useState<WaveformFrame | null>(null);
  const [loading, setLoading] = useState(false);
  const detailReadyRef = useRef(false);
  const inFlightRef = useRef(false);
  const queuedRef = useRef<{ includeDetail: boolean; position: number } | null>(
    null,
  );
  const requestIdRef = useRef(0);
  const positionRef = useRef(positionSecs);
  positionRef.current = positionSecs;

  const renderLane = useCallback(
    async (includeDetail: boolean, position: number) => {
      if ((!trackId && !path) || width <= 0 || height <= 0) {
        setFrame(null);
        return;
      }

      if (inFlightRef.current) {
        queuedRef.current = { includeDetail, position };
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
          bufferRatio: BUFFER_RATIO,
          includeDetail,
          eqLowDb: eq.low,
          eqMidDb: eq.mid,
          eqHighDb: eq.high,
        });

        if (requestId === requestIdRef.current) {
          setFrame(nextFrame);
          if (includeDetail) {
            detailReadyRef.current = true;
          }
        }
      } catch (err) {
        console.error("render_waveform_lane failed", err);
        if (requestId === requestIdRef.current) {
          setFrame(null);
        }
      } finally {
        inFlightRef.current = false;
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
        const queued = queuedRef.current;
        if (queued) {
          queuedRef.current = null;
          void renderLane(queued.includeDetail, queued.position);
        }
      }
    },
    [trackId, path, width, height, eq.low, eq.mid, eq.high],
  );

  useEffect(() => {
    detailReadyRef.current = false;
    void renderLane(false, positionRef.current).then(() =>
      renderLane(true, positionRef.current),
    );
  }, [trackId, path, width, height, renderLane]);

  useEffect(() => {
    if (!playing) {
      void renderLane(detailReadyRef.current, positionSecs);
      return;
    }

    let frameId = 0;
    let lastFetch = 0;
    const tick = (now: number) => {
      if (now - lastFetch >= PLAYING_REFRESH_MS) {
        lastFetch = now;
        void renderLane(true, positionRef.current);
      }
      frameId = window.requestAnimationFrame(tick);
    };
    frameId = window.requestAnimationFrame(tick);

    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [playing, positionSecs, renderLane]);

  return { frame, loading };
}
