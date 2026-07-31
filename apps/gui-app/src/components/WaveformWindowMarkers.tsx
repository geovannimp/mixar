import type { DeckActiveLoop, DeckHotCueMarker } from "@/types";

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
  windowStartMs: number;
  windowEndMs: number;
  hotCues?: DeckHotCueMarker[];
  activeLoop?: DeckActiveLoop | null;
}

function toWindowPercent(secs: number, windowStartMs: number, windowEndMs: number): number {
  const span = windowEndMs - windowStartMs;
  if (span <= 0) {
    return 0;
  }
  return Math.min(100, Math.max(0, ((secs - windowStartMs) / span) * 100));
}

export function WaveformWindowMarkers({
  windowStartMs,
  windowEndMs,
  hotCues = [],
  activeLoop = null,
}: WaveformWindowMarkersProps) {
  const span = windowEndMs - windowStartMs;
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
            left: `${toWindowPercent(activeLoop.in_ms, windowStartMs, windowEndMs)}%`,
            width: `${Math.max(
              0,
              toWindowPercent(activeLoop.out_ms, windowStartMs, windowEndMs) -
                toWindowPercent(activeLoop.in_ms, windowStartMs, windowEndMs),
            )}%`,
          }}
        />
      ) : null}

      {hotCues.map((cue) => {
        const left = toWindowPercent(cue.position_ms, windowStartMs, windowEndMs);
        if (left < 0 || left > 100) {
          return null;
        }
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
