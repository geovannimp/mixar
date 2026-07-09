import { type MotionValue, useMotionValueEvent } from "motion/react";
import { useEffect, useState } from "react";
import type { DeckActiveLoop, DeckHotCueMarker } from "../types";
import { WaveformWindowMarkers } from "./WaveformWindowMarkers";

interface WaveformWindowMarkersMotionProps {
  motionPos: MotionValue<number>;
  visibleSecs: number;
  hotCues?: DeckHotCueMarker[];
  activeLoop?: DeckActiveLoop | null;
}

export function WaveformWindowMarkersMotion({
  motionPos,
  visibleSecs,
  hotCues,
  activeLoop,
}: WaveformWindowMarkersMotionProps) {
  const [window, setWindow] = useState(() => ({
    start: motionPos.get() - visibleSecs / 2,
    end: motionPos.get() + visibleSecs / 2,
  }));

  useMotionValueEvent(motionPos, "change", (center) => {
    setWindow({
      start: center - visibleSecs / 2,
      end: center + visibleSecs / 2,
    });
  });

  useEffect(() => {
    const center = motionPos.get();
    setWindow({
      start: center - visibleSecs / 2,
      end: center + visibleSecs / 2,
    });
  }, [motionPos, visibleSecs]);

  return (
    <WaveformWindowMarkers
      windowStartSecs={window.start}
      windowEndSecs={window.end}
      hotCues={hotCues}
      activeLoop={activeLoop}
    />
  );
}
