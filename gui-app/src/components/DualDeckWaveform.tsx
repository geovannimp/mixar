import { DECK_ACCENTS } from "../lib/ui";
import { useRenderWaveformLane } from "../hooks/useRenderWaveformLane";
import type { DeckStatus } from "../types";
import { RustRenderedLane, useLaneDimensions } from "./RustRenderedLane";

interface DualDeckWaveformProps {
  decks: DeckStatus[];
}

function WaveformLane({
  deck,
  accent,
}: {
  deck: DeckStatus;
  accent: (typeof DECK_ACCENTS)["a"];
}) {
  const { ref, size } = useLaneDimensions();
  const positionSecs = deck.position_secs ?? 0;

  const { frame, estimatedPosition } = useRenderWaveformLane({
    trackId: deck.track_id,
    path: deck.track,
    positionSecs,
    playing: deck.playing,
    eq: deck.eq,
    width: size.width,
    height: size.height,
  });

  return (
    <div ref={ref} className="relative min-h-0 flex-1">
      <RustRenderedLane
        frame={frame}
        positionSecs={positionSecs}
        playing={deck.playing}
        estimatedPosition={estimatedPosition}
        label={accent.label}
        labelClass={accent.text}
      />
    </div>
  );
}

export function DualDeckWaveform({ decks }: DualDeckWaveformProps) {
  const deckA = decks[0];
  const deckB = decks[1] ?? decks[0];

  return (
    <div className="relative flex h-full min-h-0 flex-col overflow-hidden border-b border-white/10 bg-black">
      <WaveformLane deck={deckA} accent={DECK_ACCENTS.a} />
      <div className="h-px shrink-0 bg-white/10" aria-hidden />
      <WaveformLane deck={deckB} accent={DECK_ACCENTS.b} />

      <div
        className="pointer-events-none absolute inset-y-0 left-1/2 z-20 w-px -translate-x-1/2 bg-white/90 shadow-[0_0_8px_rgba(255,255,255,0.45)]"
        aria-hidden
      />

      {!deckA.track && !deckB.track ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/60 text-xs font-medium uppercase tracking-widest text-zinc-500">
          Load tracks to see waveforms
        </div>
      ) : null}
    </div>
  );
}
