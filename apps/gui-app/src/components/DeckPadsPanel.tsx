import { match } from "ts-pattern";
import { BeatJumpPads } from "@/components/deck-pads/BeatJumpPads";
import { HotCuePads } from "@/components/deck-pads/HotCuePads";
import { LoopRollPads } from "@/components/deck-pads/LoopRollPads";
import { SamplerPads } from "@/components/deck-pads/SamplerPads";
import { cn } from "@/lib/utils";
import { PAD_MODES, PAD_MODE_SHORT_LABELS } from "@/lib/padModes";
import type {
  DeckHotCueMarker,
  DeckStatus,
  PadMode,
  SamplerBankInfo,
  SamplerPlayMode,
  SamplerSlotInfo,
} from "@/types";

interface DeckPadsPanelProps {
  deck: DeckStatus;
  samplerSlots: SamplerSlotInfo[];
  samplerBanks: SamplerBankInfo[];
  effectivePlayMode: SamplerPlayMode;
  disabled?: boolean;
  onSetPadMode: (mode: PadMode) => void;
  onTriggerHotCue: (cue: DeckHotCueMarker) => void;
  onSaveHotCue: (slot: number) => void;
  onDeleteHotCue: (slot: number) => void;
  onBeginLoopRoll: (beats: number) => void;
  onEndLoopRoll: () => void;
  onBeatJump: (beats: number) => void;
  onTriggerSampler: (slot: number) => void;
  onEndSampler: (slot: number) => void;
  onClearSamplerSlot: (slot: number) => void;
  onSelectSamplerBank: (bankId: string) => void;
  onSaveSamplerBank: (bankId: string, name: string, playMode: SamplerPlayMode | null) => void;
}

export function DeckPadsPanel({
  deck,
  samplerSlots,
  samplerBanks,
  effectivePlayMode,
  disabled,
  onSetPadMode,
  onTriggerHotCue,
  onSaveHotCue,
  onDeleteHotCue,
  onBeginLoopRoll,
  onEndLoopRoll,
  onBeatJump,
  onTriggerSampler,
  onEndSampler,
  onClearSamplerSlot,
  onSelectSamplerBank,
  onSaveSamplerBank,
}: DeckPadsPanelProps) {
  const hasTrack = Boolean(deck.track);
  const controlsDisabled = disabled || !hasTrack;
  const holdLike = effectivePlayMode === "hold" || effectivePlayMode === "loop";

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 shadow-inner">
      <div
        role="tablist"
        aria-label="Pad mode"
        className="grid shrink-0 grid-cols-4 border-b border-white/8"
      >
        {PAD_MODES.map((mode) => {
          const active = deck.pad_mode === mode;
          return (
            <button
              key={mode}
              type="button"
              role="tab"
              aria-selected={active}
              disabled={disabled}
              title={PAD_MODE_SHORT_LABELS[mode]}
              className={cn(
                "px-1 py-1.5 text-[9px] font-bold uppercase tracking-[0.14em] transition-colors sm:text-[10px] sm:tracking-[0.18em]",
                active
                  ? "bg-white/10 text-zinc-100"
                  : "text-zinc-500 hover:bg-white/5 hover:text-zinc-300",
                "disabled:cursor-not-allowed disabled:text-zinc-600 disabled:hover:bg-transparent",
              )}
              onClick={() => onSetPadMode(mode)}
            >
              {PAD_MODE_SHORT_LABELS[mode]}
            </button>
          );
        })}
      </div>

      {match(deck.pad_mode)
        .with("hot_cue", () => (
          <HotCuePads
            hotCues={deck.hot_cues}
            disabled={controlsDisabled}
            onTrigger={onTriggerHotCue}
            onSave={onSaveHotCue}
            onDelete={onDeleteHotCue}
          />
        ))
        .with("loop_roll", () => (
          <LoopRollPads
            disabled={controlsDisabled}
            onBegin={onBeginLoopRoll}
            onEnd={onEndLoopRoll}
          />
        ))
        .with("beat_jump", () => (
          <BeatJumpPads disabled={controlsDisabled} onBeatJump={onBeatJump} />
        ))
        .with("sampler", () => (
          <SamplerPads
            deckId={deck.id}
            slots={samplerSlots}
            banks={samplerBanks}
            activeBankId={deck.active_sampler_bank_id}
            disabled={disabled}
            holdLike={holdLike}
            effectivePlayMode={effectivePlayMode}
            onTrigger={onTriggerSampler}
            onEnd={onEndSampler}
            onClear={onClearSamplerSlot}
            onSelectBank={onSelectSamplerBank}
            onSaveBank={onSaveSamplerBank}
          />
        ))
        .exhaustive()}
    </div>
  );
}
