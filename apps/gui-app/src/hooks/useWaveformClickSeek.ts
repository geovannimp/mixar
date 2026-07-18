import { useCallback } from "react";

export interface WaveformClickSeekConfig {
  enabled: boolean;
  durationSecs: number;
  onSeek: (positionSecs: number) => void;
}

export function useWaveformClickSeek({ enabled, durationSecs, onSeek }: WaveformClickSeekConfig) {
  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (!enabled || durationSecs <= 0) {
        return;
      }
      const rect = event.currentTarget.getBoundingClientRect();
      const fraction = (event.clientX - rect.left) / Math.max(rect.width, 1);
      const position = Math.min(durationSecs, Math.max(0, fraction * durationSecs));
      onSeek(position);
    },
    [durationSecs, enabled, onSeek],
  );

  return {
    handlers: { onClick: handleClick },
    cursorClass: enabled ? "cursor-pointer" : "",
  };
}
