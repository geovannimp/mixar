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
  channelAccent: DeckAccent;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  volume: number;
  eq: DeckEq;
  disabled?: boolean;
  compact?: boolean;
  onVolumeChange: (volume: number) => void;
  onEqChange: (eq: DeckEq) => void;
}

function DeckChannelStrip({
  label,
  channelAccent,
  accent,
  volume,
  eq,
  disabled,
  compact,
  onVolumeChange,
  onEqChange,
}: DeckChannelStripProps) {
  const percent = Math.round(volume * 100);
  const channelWidth = compact ? "w-12" : CHANNEL_WIDTH_CLASS;

  return (
    <div
      className={`flex ${compact ? "h-auto" : "h-full"} ${channelWidth} shrink-0 flex-col items-center gap-1`}
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
        className={`flex ${compact ? `h-20 ${FADER_WIDTH_CLASS}` : `min-h-0 flex-1 ${FADER_WIDTH_CLASS}`} items-center justify-center py-1 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center`}
      >
        <Slider
          orientation="vertical"
          thumbAlignment="center"
          thumbVariant="fader"
          channelAccent={channelAccent}
          showIndicator
          min={0}
          max={100}
          value={percent}
          disabled={disabled}
          aria-label={`${label} volume`}
          className={`${compact ? "h-20" : "h-full"} ${FADER_WIDTH_CLASS}`}
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
          showIndicator={false}
          thumbVariant="fader"
          crossfaderTrack
          min={0}
          max={100}
          value={percent}
          disabled={disabled}
          aria-label="Crossfader"
          className="min-w-0 flex-1 [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:min-w-0"
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
  layout?: "full" | "compact";
  onVolumeChange: (deckId: number, volume: number) => void;
  onEqChange: (deckId: number, eq: DeckEq) => void;
  onCrossfaderChange: (position: number) => void;
}

export function DeckMixer({
  decks,
  crossfader,
  disabled,
  layout = "full",
  onVolumeChange,
  onEqChange,
  onCrossfaderChange,
}: DeckMixerProps) {
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;
  const channelAccents = ["a", "b"] as const satisfies readonly DeckAccent[];
  const labels = ["A", "B"] as const;
  const compact = layout === "compact";

  return (
    <div
      className={`flex shrink-0 flex-col gap-2 overflow-hidden bg-zinc-900/50 ${
        compact
          ? "w-full px-2 py-2"
          : "h-full min-h-0 w-[8.25rem] border-x border-white/6 px-1.5 py-3"
      }`}
    >
      <span className="shrink-0 text-center text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
        Mixer
      </span>

      <div
        className={
          compact
            ? "flex min-h-0 flex-col gap-2"
            : "flex min-h-0 flex-1 flex-col gap-2"
        }
      >
        <div
          className={
            compact
              ? "flex items-end justify-center gap-3"
              : "flex min-h-0 flex-1 items-stretch justify-center gap-1"
          }
        >
          {labels.map((label, index) => (
            <DeckChannelStrip
              key={label}
              label={label}
              channelAccent={channelAccents[index]}
              accent={accents[index]}
              volume={decks[index]?.volume ?? 1}
              eq={decks[index]?.eq ?? DEFAULT_DECK_EQ}
              disabled={disabled}
              compact={compact}
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
