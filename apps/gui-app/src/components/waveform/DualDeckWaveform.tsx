import { useCallback, type RefObject } from "react";
import { DECK_ACCENTS } from "@/lib/ui";
import { useWaveformDragScrub } from "@/hooks/useWaveformDragScrub";
import { useSmoothPlayhead } from "@/hooks/useSmoothPlayhead";
import { useDeckHasTrack } from "@/hooks/engine/useDeckHasTrack";
import { useDeckPosition } from "@/hooks/engine/useDeckPosition";
import { useDeckWaveform } from "@/hooks/engine/useDeckWaveform";
import { useEngineRunning } from "@/hooks/engine/useEngineRunning";
import { engineActions } from "@/stores/engineStore";
import { useTrack } from "@/hooks/library/useTrack";
import type { DeckActiveLoop, DeckEq } from "@/types";
import { useRenderWaveformLane } from "@/hooks/useRenderWaveformLane";
import { RustRenderedLane } from "./RustRenderedLane";
import { useLaneDimensions } from "./useLaneDimensions";
import { WaveformWindowMarkersMotion } from "./WaveformWindowMarkersMotion";

function WaveformPlayheadHost({
  deckId,
  accent,
  trackId,
  path,
  playing,
  speed,
  eq,
  activeLoop,
  durationMs,
  width,
  height,
  laneRef,
}: {
  deckId: number;
  accent: (typeof DECK_ACCENTS)["a"];
  trackId: string | null;
  path: string | null;
  playing: boolean;
  speed: number;
  eq: DeckEq;
  activeLoop: DeckActiveLoop | null;
  durationMs: number | null;
  width: number;
  height: number;
  laneRef: RefObject<HTMLDivElement | null>;
}) {
  const engineRunning = useEngineRunning();
  const positionMs = useDeckPosition(deckId);
  const { track: libraryTrack } = useTrack(trackId);
  const hasTrack = Boolean(path);
  const duration = durationMs ?? undefined;

  const playhead = useSmoothPlayhead({
    positionMs,
    playing,
    speed,
    maxMs: duration,
  });

  const { trackCache, tileRevision, visibleMs } = useRenderWaveformLane({
    trackId,
    path,
    durationMs,
    positionMs,
    playing,
    speed,
    eq,
    width,
    height,
    getPosition: playhead.getPosition,
    isScrubbing: playhead.isScrubbing,
  });
  const seekEnabled = hasTrack && engineRunning;
  const safeSpeed = Number.isFinite(speed) && speed > 0 ? Math.min(2, Math.max(0.5, speed)) : 1;
  const viewSpanMs = trackCache && width > 0 ? (width * safeSpeed) / trackCache.pxPerMs : visibleMs;

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
    playing,
    speed,
    maxMs: duration,
    onSeek: handleSeek,
    playhead,
  });

  const ariaValueNow = scrubbing ? getPosition() : positionMs;

  return (
    <div
      ref={laneRef}
      className={`relative min-h-0 flex-1 ${cursorClass}`}
      style={{ touchAction: seekEnabled ? "none" : undefined }}
      {...handlers}
      role={seekEnabled ? "slider" : undefined}
      aria-label={seekEnabled ? `${accent.label} waveform scrub` : undefined}
      aria-valuemin={0}
      aria-valuemax={durationMs ?? undefined}
      aria-valuenow={ariaValueNow}
    >
      <RustRenderedLane
        trackCache={trackCache}
        tileRevision={tileRevision}
        viewportWidth={width}
        speed={speed}
        motionPos={playhead.motionPos}
        label={accent.label}
        labelClass={accent.text}
      />
      <WaveformWindowMarkersMotion
        motionPos={playhead.motionPos}
        visibleMs={viewSpanMs}
        hotCues={libraryTrack?.hot_cues ?? []}
        activeLoop={activeLoop}
      />
    </div>
  );
}

function WaveformLane({ deckId, accent }: { deckId: number; accent: (typeof DECK_ACCENTS)["a"] }) {
  const deck = useDeckWaveform(deckId);
  const { ref, size } = useLaneDimensions();

  return (
    <WaveformPlayheadHost
      deckId={deckId}
      accent={accent}
      trackId={deck.track_id}
      path={deck.track}
      playing={deck.playing}
      speed={deck.speed}
      eq={deck.eq}
      activeLoop={deck.active_loop}
      durationMs={deck.duration_ms}
      width={size.width}
      height={size.height}
      laneRef={ref}
    />
  );
}

export function DualDeckWaveform() {
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
}
