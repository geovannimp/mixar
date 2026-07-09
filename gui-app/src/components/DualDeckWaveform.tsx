import { memo, useCallback } from "react";
import { DECK_ACCENTS } from "../lib/ui";
import { WAVEFORM_VISIBLE_SECS } from "../lib/spectralColor";
import { useWaveformDragScrub } from "../hooks/useWaveformDragScrub";
import {
  engineActions,
  useDeckHasTrack,
  useDeckWaveform,
  useEngineRunning,
} from "../hooks/useEngine";
import { useRenderWaveformLane } from "../hooks/useRenderWaveformLane";
import { RustRenderedLane, useLaneDimensions } from "./RustRenderedLane";
import { WaveformWindowMarkers } from "./WaveformWindowMarkers";

const WaveformLane = memo(function WaveformLane({
  deckId,
  accent,
}: {
  deckId: number;
  accent: (typeof DECK_ACCENTS)["a"];
}) {
  const engineRunning = useEngineRunning();
  const deck = useDeckWaveform(deckId);
  const { ref, size } = useLaneDimensions();
  const positionSecs = deck.position_secs ?? 0;
  const hasTrack = Boolean(deck.track);
  const durationSecs = deck.duration_secs ?? undefined;

  const { frame, estimatedPosition } = useRenderWaveformLane({
    trackId: deck.track_id,
    path: deck.track,
    positionSecs,
    playing: deck.playing,
    eq: deck.eq,
    width: size.width,
    height: size.height,
  });

  const visibleSecs = frame?.visible_secs ?? WAVEFORM_VISIBLE_SECS;
  const seekEnabled = hasTrack && engineRunning;

  const handleSeek = useCallback(
    (secs: number) => {
      void engineActions.seekDeck(deckId, secs);
    },
    [deckId],
  );

  const { scrubbing, displayPosition, handlers, cursorClass } =
    useWaveformDragScrub({
      enabled: seekEnabled,
      mode: "center",
      spanSecs: visibleSecs,
      positionSecs,
      estimatedPosition,
      playing: deck.playing,
      maxSecs: durationSecs,
      onSeek: handleSeek,
    });

  const viewCenterSecs = displayPosition;
  const windowStartSecs = viewCenterSecs - visibleSecs / 2;
  const windowEndSecs = viewCenterSecs + visibleSecs / 2;

  return (
    <div
      ref={ref}
      className={`relative min-h-0 flex-1 ${cursorClass}`}
      {...handlers}
      role={seekEnabled ? "slider" : undefined}
      aria-label={seekEnabled ? `${accent.label} waveform scrub` : undefined}
      aria-valuemin={0}
      aria-valuemax={deck.duration_secs ?? undefined}
      aria-valuenow={displayPosition}
    >
      <RustRenderedLane
        frame={frame}
        positionSecs={displayPosition}
        playing={deck.playing && !scrubbing}
        estimatedPosition={() => displayPosition}
        label={accent.label}
        labelClass={accent.text}
      />
      <WaveformWindowMarkers
        windowStartSecs={windowStartSecs}
        windowEndSecs={windowEndSecs}
        hotCues={deck.hot_cues}
        activeLoop={deck.active_loop}
      />
    </div>
  );
});

export const DualDeckWaveform = memo(function DualDeckWaveform() {
  const deckAHasTrack = useDeckHasTrack(0);
  const deckBHasTrack = useDeckHasTrack(1);

  return (
    <div className="relative flex h-full min-h-0 flex-col overflow-hidden border-b border-white/10 bg-black">
      <WaveformLane deckId={0} accent={DECK_ACCENTS.a} />
      <div className="h-px shrink-0 bg-white/10" aria-hidden />
      <WaveformLane deckId={1} accent={DECK_ACCENTS.b} />

      <div
        className="pointer-events-none absolute inset-y-0 left-1/2 z-20 w-px -translate-x-1/2 bg-white/90 shadow-[0_0_8px_rgba(255,255,255,0.45)]"
        aria-hidden
      />

      {!deckAHasTrack && !deckBHasTrack ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/60 text-xs font-medium uppercase tracking-widest text-zinc-500">
          Load tracks to see waveforms
        </div>
      ) : null}
    </div>
  );
});
