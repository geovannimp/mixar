import { useEffect, useRef, useState } from "react";
import type { WaveformFrame } from "../types";

interface RustRenderedLaneProps {
  frame: WaveformFrame | null;
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
  label,
  labelClass,
}: RustRenderedLaneProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !frame || frame.width <= 0 || frame.height <= 0) {
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    const dpr = window.devicePixelRatio || 1;
    const displayWidth = canvas.clientWidth;
    const displayHeight = canvas.clientHeight;
    if (displayWidth <= 0 || displayHeight <= 0) {
      return;
    }

    canvas.width = Math.floor(displayWidth * dpr);
    canvas.height = Math.floor(displayHeight * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    try {
      const rgba = decodeBase64Rgba(frame.rgba_base64);
      const expected = frame.width * frame.height * 4;
      if (rgba.length !== expected) {
        console.error(
          `waveform rgba size mismatch: got ${rgba.length}, expected ${expected}`,
        );
        return;
      }

      const image = new ImageData(rgba, frame.width, frame.height);
      const scratch = document.createElement("canvas");
      scratch.width = frame.width;
      scratch.height = frame.height;
      scratch.getContext("2d")?.putImageData(image, 0, 0);

      ctx.clearRect(0, 0, displayWidth, displayHeight);
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(scratch, 0, 0, displayWidth, displayHeight);
    } catch (err) {
      console.error("failed to paint waveform frame", err);
    }
  }, [frame]);

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
