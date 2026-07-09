import type { DeckActiveLoop, DeckHotCueMarker } from "../types";

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

interface WaveformWindowMarkersProps {
  windowStartSecs: number;
  windowEndSecs: number;
  hotCues?: DeckHotCueMarker[];
  activeLoop?: DeckActiveLoop | null;
}

function toWindowPercent(
  secs: number,
  windowStartSecs: number,
  windowEndSecs: number,
): number {
  const span = windowEndSecs - windowStartSecs;
  if (span <= 0) {
    return 0;
  }
  return Math.min(100, Math.max(0, ((secs - windowStartSecs) / span) * 100));
}

export function WaveformWindowMarkers({
  windowStartSecs,
  windowEndSecs,
  hotCues = [],
  activeLoop = null,
}: WaveformWindowMarkersProps) {
  const span = windowEndSecs - windowStartSecs;
  if (span <= 0) {
    return null;
  }

  const hasLoop = Boolean(activeLoop?.active);
  const hasHotCues = hotCues.length > 0;
  if (!hasLoop && !hasHotCues) {
    return null;
  }

  return (
    <div className="pointer-events-none absolute inset-0 z-10" aria-hidden>
      {hasLoop && activeLoop ? (
        <div
          className="absolute inset-y-0 border-x border-emerald-400/70 bg-emerald-400/18"
          style={{
            left: `${toWindowPercent(activeLoop.in_secs, windowStartSecs, windowEndSecs)}%`,
            width: `${Math.max(
              0,
              toWindowPercent(activeLoop.out_secs, windowStartSecs, windowEndSecs) -
                toWindowPercent(activeLoop.in_secs, windowStartSecs, windowEndSecs),
            )}%`,
          }}
        />
      ) : null}

      {hotCues.map((cue) => {
        const left = toWindowPercent(cue.position_secs, windowStartSecs, windowEndSecs);
        if (left < 0 || left > 100) {
          return null;
        }
        const color =
          cue.color ??
          HOT_CUE_COLORS[cue.slot % HOT_CUE_COLORS.length] ??
          HOT_CUE_COLORS[0];

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
