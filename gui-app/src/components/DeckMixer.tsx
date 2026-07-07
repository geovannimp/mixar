import { Slider } from "@/components/ui/slider";
import { DECK_ACCENTS, type DeckAccent } from "../lib/ui";
import { DEFAULT_DECK_EQ, type DeckEq, type DeckStatus } from "../types";
import { RotaryKnob } from "./RotaryKnob";

const CHANNEL_WIDTH_CLASS = "w-14";
const FADER_WIDTH_CLASS = "w-8";

type EqBand = keyof DeckEq;

const EQ_BANDS: { id: EqBand; label: string }[] = [
  { id: "high", label: "HI" },
  { id: "mid", label: "MID" },
  { id: "low", label: "LOW" },
];

interface DeckEqKnobProps {
  label: string;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  value: number;
  disabled?: boolean;
  onValueChange: (value: number) => void;
}

function DeckEqKnob({
  label,
  accent,
  value,
  disabled,
  onValueChange,
}: DeckEqKnobProps) {
  return (
    <RotaryKnob
      label={label}
      value={value}
      disabled={disabled}
      accentClass={accent.text}
      ringClass={accent.ring}
      onValueChange={onValueChange}
    />
  );
}

interface DeckChannelStripProps {
  label: string;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  volume: number;
  eq: DeckEq;
  disabled?: boolean;
  onVolumeChange: (volume: number) => void;
  onEqChange: (eq: DeckEq) => void;
}

function DeckChannelStrip({
  label,
  accent,
  volume,
  eq,
  disabled,
  onVolumeChange,
  onEqChange,
}: DeckChannelStripProps) {
  const percent = Math.round(volume * 100);

  return (
    <div
      className={`flex h-full ${CHANNEL_WIDTH_CLASS} shrink-0 flex-col items-center gap-1`}
    >
      <span
        className={`shrink-0 text-center text-[9px] font-semibold uppercase tracking-widest ${accent.text}`}
      >
        {label}
      </span>

      <div className="flex w-full shrink-0 flex-col items-center gap-1 pb-1">
        {EQ_BANDS.map((band) => (
          <DeckEqKnob
            key={band.id}
            label={band.label}
            accent={accent}
            value={eq[band.id]}
            disabled={disabled}
            onValueChange={(next) => {
              onEqChange({ ...eq, [band.id]: next });
            }}
          />
        ))}
      </div>

      <div className="w-full shrink-0 border-t border-white/6" />

      <div
        className={`flex min-h-0 flex-1 ${FADER_WIDTH_CLASS} items-center justify-center py-1 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center [&_[data-slot=slider-thumb]]:size-3.5`}
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

      <span className="w-full shrink-0 text-center text-[9px] tabular-nums text-zinc-500">
        {percent}%
      </span>
    </div>
  );
}

interface CrossfaderProps {
  position: number;
  disabled?: boolean;
  onPositionChange: (position: number) => void;
}

function Crossfader({ position, disabled, onPositionChange }: CrossfaderProps) {
  const percent = Math.round(position * 100);

  return (
    <div className="flex w-full shrink-0 flex-col gap-1 border-t border-white/6 pt-2">
      <span className="text-center text-[8px] font-semibold uppercase tracking-widest text-zinc-600">
        Crossfader
      </span>
      <div className="flex items-center gap-1.5 px-0.5">
        <span className="w-3 shrink-0 text-center text-[8px] font-semibold text-sky-300">
          A
        </span>
        <Slider
          orientation="horizontal"
          thumbAlignment="center"
          min={0}
          max={100}
          value={percent}
          disabled={disabled}
          aria-label="Crossfader"
          className="min-w-0 flex-1 [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:min-w-0 [&_[data-slot=slider-thumb]]:size-3"
          onValueChange={(value) => {
            const next = Array.isArray(value) ? (value[0] ?? 0) : value;
            onPositionChange(next / 100);
          }}
        />
        <span className="w-3 shrink-0 text-center text-[8px] font-semibold text-rose-300">
          B
        </span>
      </div>
    </div>
  );
}

interface DeckMixerProps {
  decks: DeckStatus[];
  crossfader: number;
  disabled?: boolean;
  onVolumeChange: (deckId: number, volume: number) => void;
  onEqChange: (deckId: number, eq: DeckEq) => void;
  onCrossfaderChange: (position: number) => void;
}

export function DeckMixer({
  decks,
  crossfader,
  disabled,
  onVolumeChange,
  onEqChange,
  onCrossfaderChange,
}: DeckMixerProps) {
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;
  const labels = ["A", "B"] as const;

  return (
    <div className="flex h-full min-h-0 w-[8.25rem] shrink-0 flex-col gap-2 overflow-hidden border-x border-white/6 bg-zinc-900/50 px-1.5 py-3">
      <span className="shrink-0 text-center text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
        Mixer
      </span>

      <div className="flex min-h-0 flex-1 flex-col gap-2">
        <div className="flex min-h-0 flex-1 items-stretch justify-center gap-1">
          {labels.map((label, index) => (
            <DeckChannelStrip
              key={label}
              label={label}
              accent={accents[index]}
              volume={decks[index]?.volume ?? 1}
              eq={decks[index]?.eq ?? DEFAULT_DECK_EQ}
              disabled={disabled}
              onVolumeChange={(volume) => onVolumeChange(index, volume)}
              onEqChange={(eq) => onEqChange(index, eq)}
            />
          ))}
        </div>

        <Crossfader
          position={crossfader}
          disabled={disabled}
          onPositionChange={onCrossfaderChange}
        />
      </div>
    </div>
  );
}
