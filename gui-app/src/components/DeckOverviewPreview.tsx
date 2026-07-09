import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  DeckActiveLoop,
  DeckHotCueMarker,
  DeckLoopMarker,
  WaveformFrame,
} from "../types";
import { WaveformCueMarkers } from "./WaveformCueMarkers";

interface DeckOverviewPreviewProps {
  trackId: string | null;
  path: string | null;
  positionSecs: number;
  durationSecs: number | null;
  hotCues?: DeckHotCueMarker[];
  loops?: DeckLoopMarker[];
  activeLoop?: DeckActiveLoop | null;
  disabled?: boolean;
  onSeek?: (positionSecs: number) => void;
}

export function DeckOverviewPreview({
  trackId,
  path,
  positionSecs,
  durationSecs,
  hotCues = [],
  loops = [],
  activeLoop = null,
  disabled,
  onSeek,
}: DeckOverviewPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [frame, setFrame] = useState<WaveformFrame | null>(null);
  const [width, setWidth] = useState(0);

  const hasTrack = Boolean(trackId || path);
  const duration = durationSecs != null && durationSecs > 0 ? durationSecs : 1;

  const overlayLoops: DeckLoopMarker[] = activeLoop
    ? [
        ...loops,
        {
          start_secs: activeLoop.in_secs,
          end_secs: activeLoop.out_secs,
          active: true,
        },
      ]
    : loops;

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
      height: 32,
      positionSecs: duration / 2,
      visibleSecs: duration,
      bufferRatio: 0,
      includeDetail: false,
      includeBeatGrid: true,
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
    canvas.height = 32;
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
      ctx.clearRect(0, 0, width, 32);
      ctx.drawImage(strip, 0, 0, frame.width, frame.height, 0, 0, width, 32);
    } catch {
      ctx.clearRect(0, 0, width, 32);
    }
  }, [frame, width]);

  const handleSeek = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!onSeek || disabled || !hasTrack) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
    onSeek(ratio * duration);
  };

  const playheadPercent =
    duration > 0 ? Math.min(100, Math.max(0, (positionSecs / duration) * 100)) : 0;

  return (
    <div
      ref={containerRef}
      className={`relative h-8 shrink-0 overflow-hidden rounded border border-white/8 bg-black/40 ${
        onSeek && !disabled && hasTrack ? "cursor-pointer" : ""
      }`}
      onClick={handleSeek}
      role={onSeek && hasTrack ? "slider" : undefined}
      aria-label={onSeek && hasTrack ? "Overview waveform seek" : undefined}
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={positionSecs}
    >
      {hasTrack ? (
        <>
          <canvas ref={canvasRef} className="block h-full w-full" aria-hidden />
          <WaveformCueMarkers
            durationSecs={duration}
            hotCues={hotCues}
            loops={overlayLoops}
          />
          <div
            className="pointer-events-none absolute inset-y-0 z-20 w-px bg-white/90 shadow-[0_0_6px_rgba(255,255,255,0.45)]"
            style={{ left: `${playheadPercent}%` }}
            aria-hidden
          />
        </>
      ) : null}
    </div>
  );
}
