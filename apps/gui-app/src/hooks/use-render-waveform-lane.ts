import { useCallback, useEffect, useRef, useState } from "react";
import { getLibraryTransport } from "@/lib/library/transport";
import type { DeckEq, WaveformFrame } from "@/types";
import { type TileQuality, WaveformTrackCache } from "@/lib/waveform-track-cache";
import { waveformVisibleSourceMs } from "@/lib/spectral-color";
import { normToStripDb } from "@/lib/eq";
import { asError, waveformLogger } from "@/lib/logging";

const MAX_CONCURRENT_TILE_FETCHES = 3;
/** Settle EQ before rebuilding tiles — every CC must not nuke the cache. */
const EQ_WAVEFORM_DEBOUNCE_MS = 120;
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
  const [eqForRender, setEqForRender] = useState(eq);

  const cacheRef = useRef<WaveformTrackCache | null>(null);
  const inFlightRef = useRef(0);
  const requestIdRef = useRef(0);
  const revisionRafRef = useRef(0);

  const getPositionRef = useRef(getPosition);
  const isScrubbingRef = useRef(isScrubbing);
  const eqRef = useRef(eqForRender);
  const visibleSourceMs = waveformVisibleSourceMs(speed);
  const visibleSourceMsRef = useRef(visibleSourceMs);

  getPositionRef.current = getPosition;
  isScrubbingRef.current = isScrubbing;
  eqRef.current = eqForRender;
  visibleSourceMsRef.current = visibleSourceMs;

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setEqForRender((prev) =>
        prev.low === eq.low && prev.mid === eq.mid && prev.high === eq.high ? prev : eq,
      );
    }, EQ_WAVEFORM_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [eq, eq.low, eq.mid, eq.high]);

  const duration = durationMs != null && durationMs > 0 ? durationMs : null;

  const bumpTileRevision = useCallback(() => {
    // Coalesce rapid L0/L1 tile blits into one React update per frame.
    if (revisionRafRef.current !== 0) {
      return;
    }
    revisionRafRef.current = window.requestAnimationFrame(() => {
      revisionRafRef.current = 0;
      const cache = cacheRef.current;
      if (cache) {
        setTileRevision(cache.tileRevision);
      }
    });
  }, []);

  const fetchTile = useCallback(
    async (
      cache: WaveformTrackCache,
      tileIndex: number,
      quality: TileQuality,
      requestId: number,
    ) => {
      if ((!trackId && !path) || !cache.tryMarkPending(tileIndex, quality)) {
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
          includeDetail: quality === "detail",
          includeBeatGrid: true,
          eqLowDb: normToStripDb(eqRef.current.low),
          eqMidDb: normToStripDb(eqRef.current.mid),
          eqHighDb: normToStripDb(eqRef.current.high),
        });

        if (requestId !== requestIdRef.current) {
          cache.clearPending(tileIndex);
          return;
        }

        cache.blitTile(frame, tileIndex, quality);
        bumpTileRevision();
      } catch (err) {
        waveformLogger.error("render_waveform_lane tile failed", asError(err));
        cache.clearPending(tileIndex);
      } finally {
        inFlightRef.current = Math.max(0, inFlightRef.current - 1);
        if (inFlightRef.current === 0) {
          setLoading(false);
        }
      }
    },
    [trackId, path, height, bumpTileRevision],
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
      const requestId = requestIdRef.current;

      // Prefer L0 (overview) so first paint wins over detail upgrades.
      const needOverview = cache.missingTileIndices(viewStart, viewEnd, prefetchMargin, "overview");
      const needDetail = cache.missingTileIndices(viewStart, viewEnd, prefetchMargin, "detail");

      let started = 0;
      const startFetch = (tileIndex: number, quality: TileQuality) => {
        if (inFlightRef.current + started >= MAX_CONCURRENT_TILE_FETCHES) {
          return false;
        }
        started += 1;
        void fetchTile(cache, tileIndex, quality, requestId);
        return true;
      };

      for (const tileIndex of needOverview) {
        if (!startFetch(tileIndex, "overview")) {
          return;
        }
      }
      for (const tileIndex of needDetail) {
        // Skip tiles still waiting on overview — they'll upgrade after L0 lands.
        if (cache.qualityOf(tileIndex) == null) {
          continue;
        }
        if (!startFetch(tileIndex, "detail")) {
          return;
        }
      }
    },
    [duration, fetchTile],
  );

  useEffect(() => {
    requestIdRef.current += 1;
    cacheRef.current = null;
    setTrackCache(null);
    setTileRevision(0);
    if (revisionRafRef.current !== 0) {
      window.cancelAnimationFrame(revisionRafRef.current);
      revisionRafRef.current = 0;
    }

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
    const initialTiles = cache.missingTileIndices(viewStart, viewEnd, 0, "overview");

    for (const tileIndex of initialTiles.slice(0, MAX_CONCURRENT_TILE_FETCHES)) {
      void fetchTile(cache, tileIndex, "overview", requestId);
    }
  }, [
    trackId,
    path,
    width,
    height,
    duration,
    eqForRender.low,
    eqForRender.mid,
    eqForRender.high,
    fetchTile,
  ]);

  useEffect(() => {
    if (playing) {
      return;
    }
    ensureVisibleTiles(0);
  }, [playing, positionMs, tileRevision, ensureVisibleTiles]);

  useEffect(() => {
    if (!playing) {
      return;
    }

    let frameId = 0;
    let lastCheck = 0;
    const tick = (now: number) => {
      // Tile fetch is not frame-critical; ~10 Hz is enough while playing.
      if (now - lastCheck >= 100) {
        lastCheck = now;
        ensureVisibleTiles(1);
      }
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
