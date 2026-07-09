import { type MotionValue } from "motion/react";
import { useEffect, useRef, useState } from "react";
import type { WaveformFrame } from "../types";

interface RustRenderedLaneProps {
  frame: WaveformFrame | null;
  motionPos: MotionValue<number>;
  label: string;
  labelClass: string;
}

function decodeBase64Rgba(base64: string): Uint8ClampedArray {
  const binary = atob(base64);
  const bytes = new Uint8ClampedArray(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

export function RustRenderedLane({
  frame,
  motionPos,
  label,
  labelClass,
}: RustRenderedLaneProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const stripRef = useRef<HTMLCanvasElement | null>(null);
  const frameRef = useRef<WaveformFrame | null>(null);
  const motionPosRef = useRef(motionPos);

  motionPosRef.current = motionPos;

  useEffect(() => {
    frameRef.current = frame;
    if (!frame || frame.width <= 0 || frame.height <= 0) {
      stripRef.current = null;
      return;
    }

    try {
      const rgba = decodeBase64Rgba(frame.rgba_base64);
      const expected = frame.width * frame.height * 4;
      if (rgba.length !== expected) {
        console.error(
          `waveform rgba size mismatch: got ${rgba.length}, expected ${expected}`,
        );
        stripRef.current = null;
        return;
      }

      const image = new ImageData(rgba, frame.width, frame.height);
      const strip = document.createElement("canvas");
      strip.width = frame.width;
      strip.height = frame.height;
      strip.getContext("2d")?.putImageData(image, 0, 0);
      stripRef.current = strip;
    } catch (err) {
      console.error("failed to decode waveform frame", err);
      stripRef.current = null;
    }
  }, [frame]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }

    let frameId = 0;
    const paint = () => {
      const ctx = canvas.getContext("2d");
      const strip = stripRef.current;
      const stripFrame = frameRef.current;
      if (!ctx) {
        frameId = window.requestAnimationFrame(paint);
        return;
      }

      const dpr = window.devicePixelRatio || 1;
      const displayWidth = canvas.clientWidth;
      const displayHeight = canvas.clientHeight;
      if (displayWidth <= 0 || displayHeight <= 0) {
        frameId = window.requestAnimationFrame(paint);
        return;
      }

      const targetW = Math.floor(displayWidth * dpr);
      const targetH = Math.floor(displayHeight * dpr);
      if (canvas.width !== targetW || canvas.height !== targetH) {
        canvas.width = targetW;
        canvas.height = targetH;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, displayWidth, displayHeight);
      ctx.fillStyle = "#050508";
      ctx.fillRect(0, 0, displayWidth, displayHeight);

      if (strip && stripFrame && stripFrame.visible_secs > 0) {
        const coverSecs =
          stripFrame.cover_end_secs - stripFrame.cover_start_secs;
        if (coverSecs > 0) {
          const viewPos = motionPosRef.current.get();
          const pxPerSec = stripFrame.width / coverSecs;
          const viewStart = viewPos - stripFrame.visible_secs / 2;
          const srcX = (viewStart - stripFrame.cover_start_secs) * pxPerSec;
          const srcW =
            (stripFrame.visible_secs / coverSecs) * stripFrame.width;

          ctx.imageSmoothingEnabled = false;
          ctx.drawImage(
            strip,
            srcX,
            0,
            srcW,
            stripFrame.height,
            0,
            0,
            displayWidth,
            displayHeight,
          );
        }
      }

      frameId = window.requestAnimationFrame(paint);
    };

    frameId = window.requestAnimationFrame(paint);
    return () => {
      window.cancelAnimationFrame(frameId);
    };
  }, []);

  return (
    <div className="absolute inset-0 bg-black">
      <canvas ref={canvasRef} className="absolute inset-0 size-full" aria-hidden />
      <span
        className={`pointer-events-none absolute left-2 top-1 z-10 text-[10px] font-bold uppercase tracking-widest ${labelClass}`}
      >
        {label}
      </span>
    </div>
  );
}

export function useLaneDimensions() {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const node = ref.current;
    if (!node) {
      return;
    }

    const update = () => {
      setSize({
        width: Math.floor(node.clientWidth),
        height: Math.floor(node.clientHeight),
      });
    };

    const observer = new ResizeObserver(update);
    observer.observe(node);
    update();

    return () => observer.disconnect();
  }, []);

  return { ref, size };
}
