import { Slider } from "@/components/ui/slider";
import { DECK_ACCENTS, type DeckAccent } from "../lib/ui";
import type { DeckStatus } from "../types";

const FADER_WIDTH_CLASS = "w-8";

interface DeckVolumeFaderProps {
  label: string;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  volume: number;
  disabled?: boolean;
  onVolumeChange: (volume: number) => void;
}

function DeckVolumeFader({
  label,
  accent,
  volume,
  disabled,
  onVolumeChange,
}: DeckVolumeFaderProps) {
  const percent = Math.round(volume * 100);

  return (
    <div
      className={`grid h-full ${FADER_WIDTH_CLASS} shrink-0 grid-rows-[auto_minmax(0,1fr)_auto] justify-items-center gap-1`}
    >
      <span
        className={`text-center text-[9px] font-semibold uppercase tracking-widest ${accent.text}`}
      >
        {label}
      </span>

      <div
        className={`flex min-h-0 ${FADER_WIDTH_CLASS} items-center justify-center py-1 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center [&_[data-slot=slider-thumb]]:size-3.5`}
      >
        <Slider
          orientation="vertical"
          thumbAlignment="center"
          min={0}
          max={100}
          value={percent}
          disabled={disabled}
          aria-label={`${label} volume`}
          className={`h-full ${FADER_WIDTH_CLASS}`}
          onValueChange={(value) => {
            const next = Array.isArray(value) ? (value[0] ?? 0) : value;
            onVolumeChange(next / 100);
          }}
        />
      </div>

      <span className="w-full text-center text-[9px] tabular-nums text-zinc-500">
        {percent}%
      </span>
    </div>
  );
}

interface DeckMixerProps {
  decks: DeckStatus[];
  disabled?: boolean;
  onVolumeChange: (deckId: number, volume: number) => void;
}

export function DeckMixer({ decks, disabled, onVolumeChange }: DeckMixerProps) {
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;
  const labels = ["A", "B"] as const;

  return (
    <div className="flex h-full min-h-0 w-[4.25rem] shrink-0 flex-col gap-2 overflow-hidden border-x border-white/6 bg-zinc-900/50 px-1.5 py-3">
      <span className="shrink-0 text-center text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
        Mixer
      </span>

      <div className="flex min-h-0 flex-1 items-stretch justify-center gap-1">
        {labels.map((label, index) => (
          <DeckVolumeFader
            key={label}
            label={label}
            accent={accents[index]}
            volume={decks[index]?.volume ?? 1}
            disabled={disabled}
            onVolumeChange={(volume) => onVolumeChange(index, volume)}
          />
        ))}
      </div>
    </div>
  );
}
