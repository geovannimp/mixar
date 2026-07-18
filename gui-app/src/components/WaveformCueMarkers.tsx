import type { DeckHotCueMarker, DeckLoopMarker } from "@/types";

const HOT_CUE_COLORS = [
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#06b6d4",
  "#3b82f6",
  "#a855f7",
  "#ec4899",
] as const;

interface WaveformCueMarkersProps {
  durationSecs: number;
  hotCues?: DeckHotCueMarker[];
  loops?: DeckLoopMarker[];
}

function toPercent(secs: number, durationSecs: number): number {
  if (durationSecs <= 0) {
    return 0;
  }
  return Math.min(100, Math.max(0, (secs / durationSecs) * 100));
}

export function WaveformCueMarkers({
  durationSecs,
  hotCues = [],
  loops = [],
}: WaveformCueMarkersProps) {
  if (durationSecs <= 0 || (hotCues.length === 0 && loops.length === 0)) {
    return null;
  }

  return (
    <div className="pointer-events-none absolute inset-0 z-10" aria-hidden>
      {loops.map((loop, index) => {
        const left = toPercent(loop.start_secs, durationSecs);
        const right = toPercent(loop.end_secs, durationSecs);
        const width = Math.max(0, right - left);
        if (width <= 0) {
          return null;
        }

        return (
          <div
            key={`loop-${index}`}
            className={`absolute inset-y-0 border-x ${
              loop.active ? "border-emerald-400/70 bg-emerald-400/18" : "border-white/20 bg-white/8"
            }`}
            style={{ left: `${left}%`, width: `${width}%` }}
          />
        );
      })}

      {hotCues.map((cue) => {
        const left = toPercent(cue.position_secs, durationSecs);
        const color =
          cue.color ?? HOT_CUE_COLORS[cue.slot % HOT_CUE_COLORS.length] ?? HOT_CUE_COLORS[0];

        return (
          <div
            key={`hotcue-${cue.slot}`}
            className="absolute top-0 h-full w-px"
            style={{ left: `${left}%`, backgroundColor: color }}
          >
            <div
              className="absolute -top-px left-1/2 size-1.5 -translate-x-1/2 rotate-45 border border-black/40"
              style={{ backgroundColor: color }}
            />
          </div>
        );
      })}
    </div>
  );
}
