import { Headphones } from "lucide-react";
import { Slider } from "@/components/ui/slider";
import { cn } from "@/lib/utils";
import { EQ_MAX_DB, EQ_MIN_DB } from "@/lib/eq";
import { buttonIcon, DECK_ACCENTS, type DeckAccent } from "@/lib/ui";
import { DEFAULT_DECK_EQ, type DeckEq, type LevelMeterMode } from "@/types";
import { useCrossfader } from "@/hooks/engine/useCrossfader";
import { useDeckMixerChannel } from "@/hooks/engine/useDeckMixerChannel";
import { useLevelMeterMode } from "@/hooks/engine/useLevelMeterMode";
import { engineActions } from "@/stores/engineStore";
import { LevelMeter } from "./LevelMeter";
import { RotaryKnob } from "./RotaryKnob";

const EQ_COLUMN_CLASS = "w-12";
const FADER_COLUMN_CLASS = "w-10";

type EqBand = keyof DeckEq;

const EQ_BANDS: { id: EqBand; label: string }[] = [
  { id: "high", label: "HI" },
  { id: "mid", label: "MID" },
  { id: "low", label: "LOW" },
];

interface MixerKnobProps {
  label: string;
  value: number;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  disabled?: boolean;
  min?: number;
  max?: number;
  onValueChange: (value: number) => void;
}

function MixerKnob({ label, value, accent, disabled, min, max, onValueChange }: MixerKnobProps) {
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
    <div className={`flex h-full ${EQ_COLUMN_CLASS} shrink-0 flex-col items-center gap-1`}>
      {EQ_BANDS.map((band) => (
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
        min={EQ_MIN_DB}
        max={EQ_MAX_DB}
        onValueChange={onFilterChange}
      />
    </div>
  );
}

interface DeckVolumeFaderProps {
  channelAccent: DeckAccent;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  gainDb: number;
  volume: number;
  cue: boolean;
  disabled?: boolean;
  onGainChange: (gainDb: number) => void;
  onVolumeChange: (volume: number) => void;
  onCueChange: (cue: boolean) => void;
}

function DeckVolumeFader({
  channelAccent,
  accent,
  gainDb,
  volume,
  cue,
  disabled,
  onGainChange,
  onVolumeChange,
  onCueChange,
}: DeckVolumeFaderProps) {
  const percent = Math.round(volume * 100);

  return (
    <div className={`flex h-full ${FADER_COLUMN_CLASS} shrink-0 flex-col items-center gap-1`}>
      <MixerKnob
        label="GAIN"
        value={gainDb}
        accent={accent}
        disabled={disabled}
        min={EQ_MIN_DB}
        max={EQ_MAX_DB}
        onValueChange={onGainChange}
      />

      <div className="flex min-h-0 w-full flex-1 items-center justify-center border-t border-white/6 py-1 **:data-[slot=slider-control]:h-full **:data-[slot=slider-control]:min-h-0 **:data-[slot=slider-control]:items-center">
        <Slider
          orientation="vertical"
          thumbAlignment="center"
          thumbVariant="fader"
          channelAccent={channelAccent}
          showIndicator
          showMarkers
          min={0}
          max={100}
          value={percent}
          disabled={disabled}
          aria-label="Volume"
          className={cn(FADER_COLUMN_CLASS, "h-full min-h-0 data-[orientation=vertical]:min-h-0")}
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
          cue && "border-emerald-500/45 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25",
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

/** Spacers match GAIN / % / cue so meters align with the volume slider track. */
function LevelMetersColumn({ levelMeterMode }: { levelMeterMode: LevelMeterMode }) {
  return (
    <div className="flex h-full shrink-0 flex-col items-center gap-1">
      <div className="invisible shrink-0" aria-hidden>
        <MixerKnob label="GAIN" value={0} accent={DECK_ACCENTS.a} onValueChange={() => undefined} />
      </div>
      <div className="flex min-h-0 w-full flex-1 items-stretch justify-center gap-0.5 border-t border-transparent px-0.5 py-1">
        <LevelMeter deckId={0} mode={levelMeterMode} />
        <LevelMeter deckId={1} mode={levelMeterMode} />
      </div>
      <span className="invisible w-full shrink-0 text-center text-[9px] tabular-nums" aria-hidden>
        100%
      </span>
      <div className="invisible size-7 shrink-0" aria-hidden />
    </div>
  );
}

function Crossfader({
  position,
  disabled,
  onPositionChange,
}: {
  position: number;
  disabled?: boolean;
  onPositionChange: (position: number) => void;
}) {
  const percent = Math.round(position * 10000) / 100;

  return (
    <div className="flex w-full shrink-0 flex-col gap-1 border-t border-white/6 pt-2">
      <span className="text-center text-[8px] font-semibold uppercase tracking-widest text-zinc-600">
        Crossfader
      </span>
      <div className="flex items-center gap-1.5 overflow-visible px-0.5">
        <span className="w-3 shrink-0 text-center text-[8px] font-semibold text-sky-300">A</span>
        <Slider
          orientation="horizontal"
          thumbAlignment="center"
          showIndicator={false}
          showMarkers
          centerNotch
          thumbVariant="fader"
          crossfaderTrack
          min={0}
          max={100}
          step={0.05}
          value={percent}
          disabled={disabled}
          aria-label="Crossfader"
          className="min-h-0 min-w-0 flex-1 overflow-visible **:data-[slot=slider-control]:min-h-0 **:data-[slot=slider-control]:min-w-0"
          onValueChange={(value) => {
            const next = Array.isArray(value) ? (value[0] ?? 0) : value;
            onPositionChange(next / 100);
          }}
        />
        <span className="w-3 shrink-0 text-center text-[8px] font-semibold text-rose-300">B</span>
      </div>
    </div>
  );
}

function MixerEqColumn({
  deckId,
  accent,
}: {
  deckId: number;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
}) {
  const channel = useDeckMixerChannel(deckId);
  return (
    <DeckEqColumn
      accent={accent}
      eq={channel.eq ?? DEFAULT_DECK_EQ}
      filterDb={channel.filter_db}
      onEqChange={(eq) => {
        void engineActions.setDeckEq(deckId, eq);
      }}
      onFilterChange={(filterDb) => {
        void engineActions.setDeckFilter(deckId, filterDb);
      }}
    />
  );
}

function MixerVolumeColumn({
  deckId,
  channelAccent,
  accent,
}: {
  deckId: number;
  channelAccent: DeckAccent;
  accent: (typeof DECK_ACCENTS)[DeckAccent];
}) {
  const channel = useDeckMixerChannel(deckId);
  return (
    <DeckVolumeFader
      channelAccent={channelAccent}
      accent={accent}
      gainDb={channel.gain_trim_db}
      volume={channel.volume}
      cue={channel.headphone_cue}
      onGainChange={(gainDb) => {
        void engineActions.setDeckGainTrim(deckId, gainDb);
      }}
      onVolumeChange={(volume) => {
        void engineActions.setDeckVolume(deckId, volume);
      }}
      onCueChange={(cue) => {
        void engineActions.setDeckHeadphoneCue(deckId, cue);
      }}
    />
  );
}

export function DeckMixer() {
  const crossfader = useCrossfader();
  const levelMeterMode = useLevelMeterMode();

  return (
    <div className="flex h-full w-max shrink-0 flex-col gap-2 border-x border-white/6 bg-zinc-900/50 px-2.5 py-3">
      <div className="flex shrink-0 items-center justify-center gap-1">
        <span className="text-center text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
          Mixer
        </span>
        <button
          type="button"
          className={cn(
            buttonIcon,
            "size-4 border-white/10 text-[7px] font-semibold text-zinc-500 hover:bg-zinc-800/90",
          )}
          aria-label={
            levelMeterMode === "mono"
              ? "Level meter mode: mono. Switch to stereo."
              : "Level meter mode: stereo. Switch to mono."
          }
          title={levelMeterMode === "mono" ? "Mono meters (max L/R)" : "Stereo meters (L/R)"}
          onClick={() =>
            engineActions.setLevelMeterMode(levelMeterMode === "mono" ? "stereo" : "mono")
          }
        >
          {levelMeterMode === "mono" ? "M" : "S"}
        </button>
      </div>

      <div className="flex min-h-0 flex-1 items-stretch justify-center gap-1">
        <MixerEqColumn deckId={0} accent={DECK_ACCENTS.a} />

        <div className="flex min-h-0 shrink-0 items-stretch gap-1 px-0.5">
          <MixerVolumeColumn deckId={0} channelAccent="a" accent={DECK_ACCENTS.a} />
          <LevelMetersColumn levelMeterMode={levelMeterMode} />
          <MixerVolumeColumn deckId={1} channelAccent="b" accent={DECK_ACCENTS.b} />
        </div>

        <MixerEqColumn deckId={1} accent={DECK_ACCENTS.b} />
      </div>

      <Crossfader
        position={crossfader}
        onPositionChange={(position) => {
          void engineActions.setCrossfader(position);
        }}
      />
    </div>
  );
}
