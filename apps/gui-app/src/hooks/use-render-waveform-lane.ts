import { useCallback, useEffect, useRef, useState } from "react";
import { getLibraryTransport } from "@/lib/library/transport";
import type { DeckEq, WaveformFrame } from "@/types";
import { WaveformTrackCache } from "@/lib/waveform-track-cache";
import { waveformVisibleSourceMs } from "@/lib/spectral-color";

const MAX_CONCURRENT_TILE_FETCHES = 3;
const libraryTransport = getLibraryTransport();

interface UseRenderWaveformLaneOptions {
  trackId: string | null;
  path: string | null;
  durationMs: number | null | undefined;
  positionMs: number;
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
  durationMs,
  positionMs,
  playing,
  speed = 1,
  eq,
  width,
  height,
  getPosition,
  isScrubbing,
}: UseRenderWaveformLaneOptions) {
  const [trackCache, setTrackCache] = useState<WaveformTrackCache | null>(null);
  const [tileRevision, setTileRevision] = useState(0);
  const [loading, setLoading] = useState(false);

  const cacheRef = useRef<WaveformTrackCache | null>(null);
  const inFlightRef = useRef(0);
  const requestIdRef = useRef(0);

  const getPositionRef = useRef(getPosition);
  const isScrubbingRef = useRef(isScrubbing);
  const eqRef = useRef(eq);
  const visibleSourceMs = waveformVisibleSourceMs(speed);
  const visibleSourceMsRef = useRef(visibleSourceMs);

  getPositionRef.current = getPosition;
  isScrubbingRef.current = isScrubbing;
  eqRef.current = eq;
  visibleSourceMsRef.current = visibleSourceMs;

  const duration = durationMs != null && durationMs > 0 ? durationMs : null;

  const fetchTile = useCallback(
    async (cache: WaveformTrackCache, tileIndex: number, requestId: number) => {
      if ((!trackId && !path) || !cache.tryMarkPending(tileIndex)) {
        return;
      }

      inFlightRef.current += 1;
      setLoading(true);

      const { start, duration: tileDuration } = cache.tileRange(tileIndex);
      const tileWidth = cache.tileWidthPx(tileIndex);

      try {
        const frame: WaveformFrame = await libraryTransport.renderWaveformLane({
          trackId,
          path: trackId ? null : path,
          width: tileWidth,
          height,
          positionMs: start + tileDuration / 2,
          visibleMs: tileDuration,
          bufferRatio: 0,
          includeDetail: true,
          includeBeatGrid: true,
          eqLowDb: eqRef.current.low,
          eqMidDb: eqRef.current.mid,
          eqHighDb: eqRef.current.high,
        });

        if (requestId !== requestIdRef.current) {
          cache.clearPending(tileIndex);
          return;
        }

        cache.blitTile(frame, tileIndex);
        setTileRevision(cache.tileRevision);
      } catch (err) {
        console.error("render_waveform_lane tile failed", err);
        cache.clearPending(tileIndex);
      } finally {
        inFlightRef.current = Math.max(0, inFlightRef.current - 1);
        if (inFlightRef.current === 0) {
          setLoading(false);
        }
      }
    },
    [trackId, path, height],
  );

  const ensureVisibleTiles = useCallback(
    (prefetchMargin = 1) => {
      const cache = cacheRef.current;
      if (!cache || !duration) {
        return;
      }

      const position = getPositionRef.current();
      const halfWindow = visibleSourceMsRef.current / 2;
      const viewStart = position - halfWindow;
      const viewEnd = position + halfWindow;
      const missing = cache.missingTileIndices(viewStart, viewEnd, prefetchMargin);
      const requestId = requestIdRef.current;

      let started = 0;
      for (const tileIndex of missing) {
        if (inFlightRef.current + started >= MAX_CONCURRENT_TILE_FETCHES) {
          break;
        }
        started += 1;
        void fetchTile(cache, tileIndex, requestId);
      }
    },
    [duration, fetchTile],
  );

  useEffect(() => {
    requestIdRef.current += 1;
    cacheRef.current = null;
    setTrackCache(null);
    setTileRevision(0);

    if ((!trackId && !path) || width <= 0 || height <= 0 || !duration) {
      return;
    }

    const cache = WaveformTrackCache.create(width, height, duration);
    cacheRef.current = cache;
    setTrackCache(cache);

    const position = getPositionRef.current();
    const viewStart = position - cache.visibleMs / 2;
    const viewEnd = position + cache.visibleMs / 2;
    const requestId = requestIdRef.current;
    const initialTiles = cache.missingTileIndices(viewStart, viewEnd, 0);

    for (const tileIndex of initialTiles.slice(0, MAX_CONCURRENT_TILE_FETCHES)) {
      void fetchTile(cache, tileIndex, requestId);
    }
  }, [trackId, path, width, height, duration, eq.low, eq.mid, eq.high, fetchTile]);

  useEffect(() => {
    if (playing) {
      return;
    }
    ensureVisibleTiles(0);
  }, [playing, positionMs, ensureVisibleTiles]);

  useEffect(() => {
    if (!playing) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      ensureVisibleTiles(1);
      frameId = window.requestAnimationFrame(tick);
    };
    frameId = window.requestAnimationFrame(tick);

    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, [playing, ensureVisibleTiles]);

  const estimatedPosition = useCallback(() => getPositionRef.current(), []);

  return {
    trackCache,
    tileRevision,
    visibleMs: visibleSourceMs,
    estimatedPosition,
    loading,
  };
}
