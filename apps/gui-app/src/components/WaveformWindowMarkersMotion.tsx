import { type MotionValue, useMotionValueEvent } from "motion/react";
import { useEffect, useState } from "react";
import type { DeckActiveLoop, DeckHotCueMarker } from "@/types";
import { WaveformWindowMarkers } from "./WaveformWindowMarkers";

interface WaveformWindowMarkersMotionProps {
  motionPos: MotionValue<number>;
  visibleMs: number;
  hotCues?: DeckHotCueMarker[];
  activeLoop?: DeckActiveLoop | null;
}

export function WaveformWindowMarkersMotion({
  motionPos,
  visibleMs,
  hotCues,
  activeLoop,
}: WaveformWindowMarkersMotionProps) {
  const [window, setWindow] = useState(() => ({
    start: motionPos.get() - visibleMs / 2,
    end: motionPos.get() + visibleMs / 2,
  }));

  useMotionValueEvent(motionPos, "change", (center) => {
    setWindow({
      start: center - visibleMs / 2,
      end: center + visibleMs / 2,
    });
  });

  useEffect(() => {
    const center = motionPos.get();
    setWindow({
      start: center - visibleMs / 2,
      end: center + visibleMs / 2,
    });
  }, [motionPos, visibleMs]);

  return (
    <WaveformWindowMarkers
      windowStartMs={window.start}
      windowEndMs={window.end}
      hotCues={hotCues}
      activeLoop={activeLoop}
    />
  );
}
