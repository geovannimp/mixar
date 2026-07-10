import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, useTransform } from "motion/react";
import { useWaveformClickSeek } from "../hooks/useWaveformClickSeek";
import { useSmoothPlayhead } from "../hooks/useSmoothPlayhead";
import type { DeckHotCueMarker, WaveformFrame } from "../types";
import { WaveformCueMarkers } from "./WaveformCueMarkers";

const OVERVIEW_HEIGHT = 48;

interface DeckOverviewPreviewProps {
  trackId: string | null;
  path: string | null;
  positionSecs: number;
  playing?: boolean;
  speed?: number;
  durationSecs: number | null;
  hotCues?: DeckHotCueMarker[];
  disabled?: boolean;
  onSeek?: (positionSecs: number) => void;
}

export function DeckOverviewPreview({
  trackId,
  path,
  positionSecs,
  playing = false,
  speed = 1,
  durationSecs,
  hotCues = [],
  disabled,
  onSeek,
}: DeckOverviewPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [frame, setFrame] = useState<WaveformFrame | null>(null);
  const [width, setWidth] = useState(0);

  const hasTrack = Boolean(trackId || path);
  const duration = durationSecs != null && durationSecs > 0 ? durationSecs : 1;
  const seekEnabled = Boolean(onSeek) && !disabled && hasTrack;

  const playhead = useSmoothPlayhead({
    positionSecs,
    playing,
    speed,
    maxSecs: duration,
  });

  const playheadLeft = useTransform(playhead.motionPos, (value) => {
    const percent =
      duration > 0 ? Math.min(100, Math.max(0, (value / duration) * 100)) : 0;
    return `${percent}%`;
  });

  const { handlers, cursorClass } = useWaveformClickSeek({
    enabled: seekEnabled,
    durationSecs: duration,
    onSeek: onSeek ?? (() => undefined),
  });

  useEffect(() => {
    const node = containerRef.current;
    if (!node) {
      return;
    }

    const observer = new ResizeObserver((entries) => {
      const next = Math.floor(entries[0]?.contentRect.width ?? 0);
      setWidth(next);
    });
    observer.observe(node);
    setWidth(Math.floor(node.clientWidth));
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!hasTrack || width <= 0) {
      setFrame(null);
      return;
    }

    let cancelled = false;

    invoke<WaveformFrame>("render_waveform_lane", {
      trackId,
      path: trackId ? null : path,
      width,
      height: OVERVIEW_HEIGHT,
      positionSecs: duration / 2,
      visibleSecs: duration,
      bufferRatio: 0,
      includeDetail: false,
      includeBeatGrid: false,
      eqLowDb: 0,
      eqMidDb: 0,
      eqHighDb: 0,
    })
      .then((next) => {
        if (!cancelled) {
          setFrame(next);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFrame(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [hasTrack, trackId, path, width, duration]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !frame || frame.width <= 0 || frame.height <= 0) {
      return;
    }

    canvas.width = width;
    canvas.height = OVERVIEW_HEIGHT;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    try {
      const binary = atob(frame.rgba_base64);
      const rgba = new Uint8ClampedArray(binary.length);
      for (let i = 0; i < binary.length; i += 1) {
        rgba[i] = binary.charCodeAt(i);
      }
      const image = new ImageData(rgba, frame.width, frame.height);
      const strip = document.createElement("canvas");
      strip.width = frame.width;
      strip.height = frame.height;
      strip.getContext("2d")?.putImageData(image, 0, 0);
      ctx.clearRect(0, 0, width, OVERVIEW_HEIGHT);
      ctx.drawImage(
        strip,
        0,
        0,
        frame.width,
        frame.height,
        0,
        0,
        width,
        OVERVIEW_HEIGHT,
      );
    } catch {
      ctx.clearRect(0, 0, width, OVERVIEW_HEIGHT);
    }
  }, [frame, width]);

  return (
    <div
      ref={containerRef}
      className={`relative h-12 shrink-0 overflow-hidden rounded border border-white/8 bg-black/40 ${cursorClass}`}
      {...handlers}
      role={seekEnabled ? "slider" : undefined}
      aria-label={seekEnabled ? "Overview waveform seek" : undefined}
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={playhead.getPosition()}
    >
      {hasTrack ? (
        <>
          <canvas ref={canvasRef} className="block h-full w-full" aria-hidden />
          <WaveformCueMarkers
            durationSecs={duration}
            hotCues={hotCues}
          />
          <motion.div
            className="pointer-events-none absolute inset-y-0 z-20 w-px bg-white/90 shadow-[0_0_6px_rgba(255,255,255,0.45)]"
            style={{
              left: playheadLeft,
              x: "-50%",
              opacity: 0.92,
            }}
            aria-hidden
          />
        </>
      ) : null}
    </div>
  );
}
