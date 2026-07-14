import { cn } from "@/lib/utils";
import type { DeckLevels, LevelMeterMode } from "../types";

const SEGMENTS = 12;
const YELLOW_FROM = 8;
const RED_FROM = 10;

function segmentOn(level: number, indexFromBottom: number): boolean {
  const threshold = (indexFromBottom + 1) / SEGMENTS;
  return level >= threshold - 1e-6;
}

function holdSegment(hold: number): number | null {
  if (hold <= 0) return null;
  return Math.min(SEGMENTS - 1, Math.max(0, Math.ceil(hold * SEGMENTS) - 1));
}

function Ladder({
  peak,
  hold,
  className,
}: {
  peak: number;
  hold: number;
  className?: string;
}) {
  const holdIdx = holdSegment(hold);
  return (
    <div
      className={cn("flex h-full w-1.5 flex-col-reverse gap-px", className)}
      aria-hidden
    >
      {Array.from({ length: SEGMENTS }, (_, fromBottom) => {
        const on = segmentOn(peak, fromBottom);
        const isHold = holdIdx === fromBottom;
        let color = "bg-zinc-800";
        if (on || isHold) {
          if (fromBottom >= RED_FROM) color = "bg-red-500";
          else if (fromBottom >= YELLOW_FROM) color = "bg-amber-400";
          else color = "bg-emerald-500";
        }
        return (
          <div
            key={fromBottom}
            className={cn(
              "min-h-0 flex-1 rounded-[1px]",
              color,
              isHold && !on && "opacity-100 ring-1 ring-white/70",
            )}
          />
        );
      })}
    </div>
  );
}

export function LevelMeter({
  levels,
  mode,
}: {
  levels: DeckLevels;
  mode: LevelMeterMode;
}) {
  switch (mode) {
    case "mono": {
      const peak = Math.max(levels.peak_l, levels.peak_r);
      const hold = Math.max(levels.peak_hold_l, levels.peak_hold_r);
      return <Ladder peak={peak} hold={hold} />;
    }
    case "stereo":
      return (
        <div className="flex h-full gap-px">
          <Ladder peak={levels.peak_l} hold={levels.peak_hold_l} />
          <Ladder peak={levels.peak_r} hold={levels.peak_hold_r} />
        </div>
      );
    default: {
      const _exhaustive: never = mode;
      return _exhaustive;
    }
  }
}
