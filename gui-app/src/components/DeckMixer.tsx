import { Headphones } from "lucide-react";
import { useState } from "react";
import { Slider } from "@/components/ui/slider";
import { cn } from "@/lib/utils";
import { EQ_MAX_DB, EQ_MIN_DB } from "../lib/eq";
import { buttonIcon, DECK_ACCENTS, type DeckAccent } from "../lib/ui";
import { DEFAULT_DECK_EQ, type DeckEq, type DeckStatus } from "../types";
import { RotaryKnob } from "./RotaryKnob";

const EQ_COLUMN_CLASS = "w-12";
const FADER_COLUMN_CLASS = "w-10";

type EqBand = keyof DeckEq;

const EQ_BANDS_LOWER: { id: EqBand; label: string }[] = [
  { id: "mid", label: "MID" },
  { id: "low", label: "LOW" },
];

interface ChannelMixerUi {
  gainDb: number;
  filterDb: number;
  cue: boolean;
}

const DEFAULT_CHANNEL_MIXER_UI: ChannelMixerUi = {
  gainDb: 0,
  filterDb: 0,
  cue: false,
};

interface MixerKnobProps {
  label: string;
  value: number;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  disabled?: boolean;
  min?: number;
  max?: number;
  onValueChange: (value: number) => void;
}

function MixerKnob({
  label,
  value,
  accent,
  disabled,
  min,
  max,
  onValueChange,
}: MixerKnobProps) {
  return (
    <RotaryKnob
      label={label}
      value={value}
      min={min}
      max={max}
      disabled={disabled}
      accentClass={accent.text}
      ringClass={accent.ring}
      onValueChange={onValueChange}
    />
  );
}

interface MixerTopKnobRowProps {
  decks: DeckStatus[];
  accents: readonly [(typeof DECK_ACCENTS)["a"], (typeof DECK_ACCENTS)["b"]];
  channelUi: ChannelMixerUi[];
  disabled?: boolean;
  onEqChange: (deckId: number, eq: DeckEq) => void;
  onGainChange: (deckId: number, gainDb: number) => void;
}

function MixerTopKnobRow({
  decks,
  accents,
  channelUi,
  disabled,
  onEqChange,
  onGainChange,
}: MixerTopKnobRowProps) {
  const eq0 = decks[0]?.eq ?? DEFAULT_DECK_EQ;
  const eq1 = decks[1]?.eq ?? DEFAULT_DECK_EQ;

  return (
    <div className="flex shrink-0 items-start justify-center gap-0.5">
      <div className={`${EQ_COLUMN_CLASS} shrink-0`}>
        <MixerKnob
          label="HI"
          value={eq0.high}
          accent={accents[0]}
          disabled={disabled}
          onValueChange={(high) => onEqChange(0, { ...eq0, high })}
        />
      </div>

      <div className="flex shrink-0 gap-0.5 px-0.5">
        <div className={`${FADER_COLUMN_CLASS} shrink-0`}>
          <MixerKnob
            label="GAIN"
            value={channelUi[0]?.gainDb ?? 0}
            accent={accents[0]}
            min={EQ_MIN_DB}
            max={EQ_MAX_DB}
            disabled={disabled}
            onValueChange={(gainDb) => onGainChange(0, gainDb)}
          />
        </div>
        <div className={`${FADER_COLUMN_CLASS} shrink-0`}>
          <MixerKnob
            label="GAIN"
            value={channelUi[1]?.gainDb ?? 0}
            accent={accents[1]}
            min={EQ_MIN_DB}
            max={EQ_MAX_DB}
            disabled={disabled}
            onValueChange={(gainDb) => onGainChange(1, gainDb)}
          />
        </div>
      </div>

      <div className={`${EQ_COLUMN_CLASS} shrink-0`}>
        <MixerKnob
          label="HI"
          value={eq1.high}
          accent={accents[1]}
          disabled={disabled}
          onValueChange={(high) => onEqChange(1, { ...eq1, high })}
        />
      </div>
    </div>
  );
}

interface DeckEqColumnProps {
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  eq: DeckEq;
  filterDb: number;
  disabled?: boolean;
  onEqChange: (eq: DeckEq) => void;
  onFilterChange: (filterDb: number) => void;
}

function DeckEqColumn({
  accent,
  eq,
  filterDb,
  disabled,
  onEqChange,
  onFilterChange,
}: DeckEqColumnProps) {
  return (
    <div
      className={`flex h-full ${EQ_COLUMN_CLASS} shrink-0 flex-col items-center gap-1`}
    >
      <div className="flex w-full shrink-0 flex-col items-center gap-1">
        {EQ_BANDS_LOWER.map((band) => (
          <MixerKnob
            key={band.id}
            label={band.label}
            value={eq[band.id]}
            accent={accent}
            disabled={disabled}
            onValueChange={(next) => {
              onEqChange({ ...eq, [band.id]: next });
            }}
          />
        ))}
        <MixerKnob
          label="FLT"
          value={filterDb}
          accent={accent}
          disabled={disabled}
          onValueChange={onFilterChange}
        />
      </div>
    </div>
  );
}

interface DeckVolumeFaderProps {
  channelAccent: DeckAccent;
  volume: number;
  cue: boolean;
  disabled?: boolean;
  onVolumeChange: (volume: number) => void;
  onCueChange: (cue: boolean) => void;
}

function DeckVolumeFader({
  channelAccent,
  volume,
  cue,
  disabled,
  onVolumeChange,
  onCueChange,
}: DeckVolumeFaderProps) {
  const percent = Math.round(volume * 100);

  return (
    <div
      className={`flex h-full ${FADER_COLUMN_CLASS} shrink-0 flex-col items-center gap-1`}
    >
      <div className="flex min-h-0 w-full flex-1 items-center justify-center border-t border-white/6 py-1 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center">
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
          aria-label="Volume"
          className={`h-full ${FADER_COLUMN_CLASS}`}
          onValueChange={(value) => {
            const next = Array.isArray(value) ? (value[0] ?? 0) : value;
            onVolumeChange(next / 100);
          }}
        />
      </div>

      <span className="w-full shrink-0 text-center text-[9px] tabular-nums text-zinc-500">
        {percent}%
      </span>

      <button
        type="button"
        className={cn(
          buttonIcon,
          "size-7 shrink-0 border-white/10 text-zinc-400 hover:bg-zinc-800/90",
          cue &&
            "border-emerald-500/45 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25",
        )}
        disabled={disabled}
        aria-label="Cue"
        aria-pressed={cue}
        title="Headphone cue"
        onClick={() => onCueChange(!cue)}
      >
        <Headphones className="size-3.5" aria-hidden />
      </button>
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
  const channelAccents = ["a", "b"] as const satisfies readonly DeckAccent[];
  const [channelUi, setChannelUi] = useState<ChannelMixerUi[]>([
    DEFAULT_CHANNEL_MIXER_UI,
    DEFAULT_CHANNEL_MIXER_UI,
  ]);

  const updateChannelUi = (
    index: number,
    patch: Partial<ChannelMixerUi>,
  ): void => {
    setChannelUi((current) =>
      current.map((channel, channelIndex) =>
        channelIndex === index ? { ...channel, ...patch } : channel,
      ),
    );
  };

  return (
    <div className="flex h-full min-h-0 w-[12.5rem] shrink-0 flex-col gap-2 overflow-hidden border-x border-white/6 bg-zinc-900/50 px-1.5 py-3">
      <span className="shrink-0 text-center text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
        Mixer
      </span>

      <div className="flex min-h-0 flex-1 flex-col gap-1">
        <MixerTopKnobRow
          decks={decks}
          accents={accents}
          channelUi={channelUi}
          disabled={disabled}
          onEqChange={onEqChange}
          onGainChange={(deckId, gainDb) => updateChannelUi(deckId, { gainDb })}
        />

        <div className="flex min-h-0 flex-1 items-stretch justify-center gap-0.5">
          <DeckEqColumn
            accent={accents[0]}
            eq={decks[0]?.eq ?? DEFAULT_DECK_EQ}
            filterDb={channelUi[0]?.filterDb ?? 0}
            disabled={disabled}
            onEqChange={(eq) => onEqChange(0, eq)}
            onFilterChange={(filterDb) => updateChannelUi(0, { filterDb })}
          />

          <div className="flex min-h-0 shrink-0 items-stretch gap-0.5 px-0.5">
            <DeckVolumeFader
              channelAccent={channelAccents[0]}
              volume={decks[0]?.volume ?? 1}
              cue={channelUi[0]?.cue ?? false}
              disabled={disabled}
              onVolumeChange={(volume) => onVolumeChange(0, volume)}
              onCueChange={(cue) => updateChannelUi(0, { cue })}
            />
            <DeckVolumeFader
              channelAccent={channelAccents[1]}
              volume={decks[1]?.volume ?? 1}
              cue={channelUi[1]?.cue ?? false}
              disabled={disabled}
              onVolumeChange={(volume) => onVolumeChange(1, volume)}
              onCueChange={(cue) => updateChannelUi(1, { cue })}
            />
          </div>

          <DeckEqColumn
            accent={accents[1]}
            eq={decks[1]?.eq ?? DEFAULT_DECK_EQ}
            filterDb={channelUi[1]?.filterDb ?? 0}
            disabled={disabled}
            onEqChange={(eq) => onEqChange(1, eq)}
            onFilterChange={(filterDb) => updateChannelUi(1, { filterDb })}
          />
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
