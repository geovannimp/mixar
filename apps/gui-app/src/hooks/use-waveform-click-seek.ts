import { useCallback } from "react";

export interface WaveformClickSeekConfig {
  enabled: boolean;
  durationMs: number;
  onSeek: (positionMs: number) => void;
}

export function useWaveformClickSeek({ enabled, durationMs, onSeek }: WaveformClickSeekConfig) {
  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (!enabled || durationMs <= 0) {
        return;
      }
      const rect = event.currentTarget.getBoundingClientRect();
      const fraction = (event.clientX - rect.left) / Math.max(rect.width, 1);
      // Unclamped seek: map click into track time without forcing [0, duration].
      onSeek(fraction * durationMs);
    },
    [durationMs, enabled, onSeek],
  );

  return {
    handlers: { onClick: handleClick },
    cursorClass: enabled ? "cursor-pointer" : "",
  };
}
