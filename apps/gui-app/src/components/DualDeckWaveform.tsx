import { memo, useCallback } from "react";
import { DECK_ACCENTS } from "@/lib/ui";
import { useWaveformDragScrub } from "@/hooks/useWaveformDragScrub";
import { useSmoothPlayhead } from "@/hooks/useSmoothPlayhead";
import {
  engineActions,
  useDeckHasTrack,
  useDeckWaveform,
  useEngineRunning,
} from "@/hooks/useEngine";
import { useTrack } from "@/hooks/useTrack";
import { useRenderWaveformLane } from "@/hooks/useRenderWaveformLane";
import { RustRenderedLane, useLaneDimensions } from "./RustRenderedLane";
import { WaveformWindowMarkersMotion } from "./WaveformWindowMarkersMotion";

const WaveformLane = memo(function WaveformLane({
  deckId,
  accent,
}: {
  deckId: number;
  accent: (typeof DECK_ACCENTS)["a"];
}) {
  const engineRunning = useEngineRunning();
  const deck = useDeckWaveform(deckId);
  const libraryTrack = useTrack(deck.track_id);
  const { ref, size } = useLaneDimensions();
  const positionMs = deck.position_ms ?? 0;
  const hasTrack = Boolean(deck.track);
  const durationMs = deck.duration_ms ?? undefined;

  const playhead = useSmoothPlayhead({
    positionMs,
    playing: deck.playing,
    speed: deck.speed,
    maxMs: durationMs,
  });

  const { trackCache, tileRevision, visibleMs } = useRenderWaveformLane({
    trackId: deck.track_id,
    path: deck.track,
    durationMs: deck.duration_ms,
    positionMs,
    playing: deck.playing,
    speed: deck.speed,
    eq: deck.eq,
    width: size.width,
    height: size.height,
    getPosition: playhead.getPosition,
    isScrubbing: playhead.isScrubbing,
  });
  const seekEnabled = hasTrack && engineRunning;
  const safeSpeed =
    Number.isFinite(deck.speed) && deck.speed > 0 ? Math.min(2, Math.max(0.5, deck.speed)) : 1;
  // Match RustRenderedLane: viewport ms = width * speed / pxPerMs (long tracks cap density).
  const viewSpanMs =
    trackCache && size.width > 0 ? (size.width * safeSpeed) / trackCache.pxPerMs : visibleMs;

  const handleSeek = useCallback(
    (ms: number) => {
      void engineActions.seekDeck(deckId, ms);
    },
    [deckId],
  );

  const { scrubbing, getPosition, handlers, cursorClass } = useWaveformDragScrub({
    enabled: seekEnabled,
    mode: "center",
    spanMs: viewSpanMs,
    positionMs,
    playing: deck.playing,
    speed: deck.speed,
    maxMs: durationMs,
    onSeek: handleSeek,
    playhead,
  });

  const ariaValueNow = scrubbing ? getPosition() : positionMs;

  return (
    <div
      ref={ref}
      className={`relative min-h-0 flex-1 ${cursorClass}`}
      style={{ touchAction: seekEnabled ? "none" : undefined }}
      {...handlers}
      role={seekEnabled ? "slider" : undefined}
      aria-label={seekEnabled ? `${accent.label} waveform scrub` : undefined}
      aria-valuemin={0}
      aria-valuemax={deck.duration_ms ?? undefined}
      aria-valuenow={ariaValueNow}
    >
      <RustRenderedLane
        trackCache={trackCache}
        tileRevision={tileRevision}
        viewportWidth={size.width}
        speed={deck.speed}
        motionPos={playhead.motionPos}
        label={accent.label}
        labelClass={accent.text}
      />
      <WaveformWindowMarkersMotion
        motionPos={playhead.motionPos}
        visibleMs={viewSpanMs}
        hotCues={libraryTrack?.hot_cues ?? []}
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
