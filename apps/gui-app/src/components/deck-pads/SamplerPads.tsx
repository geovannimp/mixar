import { useState } from "react";
import { Settings } from "lucide-react";
import { PadGridContainer } from "@/components/deck-pads/PadGridContainer";
import { SamplerBankConfigDialog } from "@/components/SamplerBankConfigDialog";
import { TrackDropZone } from "@/components/TrackDropZone";
import { DeckButton } from "@/components/ui/deck-button";
import { DEFAULT_SAMPLER_PLAY_MODE } from "@/lib/busSettings";
import { formatDeckTimeTenth } from "@/lib/format";
import { samplerDropId } from "@/lib/trackDrag";
import { hotCueAccentForSlot } from "@/lib/ui";
import type { SamplerBankInfo, SamplerPlayMode, SamplerSlotInfo } from "@/types";

interface SamplerPadsProps {
  deckId: number;
  slots: SamplerSlotInfo[];
  banks: SamplerBankInfo[];
  activeBankId: string | null;
  disabled?: boolean;
  holdLike: boolean;
  effectivePlayMode: SamplerPlayMode;
  onTrigger: (slot: number) => void;
  onEnd: (slot: number) => void;
  onClear: (slot: number) => void;
  onSelectBank: (bankId: string) => void;
  onSaveBank: (bankId: string, name: string, playMode: SamplerPlayMode | null) => void;
}

export function SamplerPads({
  deckId,
  slots,
  banks,
  activeBankId,
  disabled,
  holdLike,
  effectivePlayMode,
  onTrigger,
  onEnd,
  onClear,
  onSelectBank,
  onSaveBank,
}: SamplerPadsProps) {
  const [bankConfigOpen, setBankConfigOpen] = useState(false);

  const activeBankIndex = banks.findIndex((bank) => bank.id === activeBankId);
  const activeBank = activeBankIndex >= 0 ? banks[activeBankIndex] : undefined;

  const cycleBank = (direction: number) => {
    if (banks.length === 0) {
      return;
    }
    const current = activeBankIndex >= 0 ? activeBankIndex : 0;
    const next = (current + direction + banks.length * 8) % banks.length;
    const bank = banks[next];
    if (bank) {
      onSelectBank(bank.id);
    }
  };

  return (
    <>
      <div className="flex shrink-0 items-center gap-1 border-b border-white/8 px-1.5 py-1">
        <button
          type="button"
          className="rounded px-1.5 py-0.5 text-[10px] text-zinc-400 hover:bg-white/5 hover:text-zinc-200 disabled:opacity-40"
          disabled={disabled || banks.length < 2}
          aria-label="Previous sampler bank"
          onClick={() => cycleBank(-1)}
        >
          ◀
        </button>
        <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-x-1.5">
          <span aria-hidden className="min-w-0" />
          <span className="max-w-[9rem] truncate text-center font-mono text-[10px] font-semibold text-zinc-200">
            {activeBank?.name ?? "No bank"}
          </span>
          <div className="flex min-w-0 items-center justify-start">
            {effectivePlayMode !== DEFAULT_SAMPLER_PLAY_MODE ? (
              <span
                className="shrink-0 rounded border border-white/12 bg-white/8 px-1 py-px text-[8px] font-bold uppercase tracking-[0.08em] text-zinc-300"
                title={
                  activeBank?.play_mode
                    ? `Play mode: ${activeBank.play_mode}`
                    : `Play mode: ${effectivePlayMode} (from settings)`
                }
              >
                {effectivePlayMode}
              </span>
            ) : null}
          </div>
        </div>
        <button
          type="button"
          className="rounded px-1.5 py-0.5 text-[10px] text-zinc-400 hover:bg-white/5 hover:text-zinc-200 disabled:opacity-40"
          disabled={disabled || banks.length < 2}
          aria-label="Next sampler bank"
          onClick={() => cycleBank(1)}
        >
          ▶
        </button>
        <button
          type="button"
          className="rounded p-1 text-zinc-400 hover:bg-white/5 hover:text-zinc-200 disabled:opacity-40"
          disabled={disabled || !activeBank}
          title="Bank settings"
          aria-label="Bank settings"
          onClick={() => setBankConfigOpen(true)}
        >
          <Settings className="size-3.5" />
        </button>
      </div>

      <SamplerBankConfigDialog
        open={bankConfigOpen}
        bank={activeBank ?? null}
        onOpenChange={setBankConfigOpen}
        onSave={onSaveBank}
      />

      <PadGridContainer>
        {Array.from({ length: 8 }, (_, slot) => {
          const sample = slots[slot];
          const filled = Boolean(sample?.path || sample?.track_id);
          const label = sample?.label?.trim();

          return (
            <TrackDropZone
              key={slot}
              id={samplerDropId(deckId, slot)}
              data={{ type: "sampler", deckId, slot }}
              disabled={disabled}
              collisionPriority={10}
              className="min-w-0"
            >
              <DeckButton
                type="button"
                size="pad"
                accent={filled ? hotCueAccentForSlot(slot) : undefined}
                disabled={disabled}
                className="w-full"
                title={
                  filled
                    ? holdLike
                      ? `Pad ${slot + 1} — hold to play, shift+click clear`
                      : `Pad ${slot + 1} — click trigger, shift+click clear, drop track to replace`
                    : `Drop a track to assign sampler pad ${slot + 1}`
                }
                onClick={(event) => {
                  if (event.shiftKey && filled) {
                    onClear(slot);
                    return;
                  }
                  if (filled && !holdLike) {
                    onTrigger(slot);
                  }
                }}
                onPointerDown={() => {
                  if (filled && holdLike) {
                    onTrigger(slot);
                  }
                }}
                onPointerUp={() => {
                  if (filled && holdLike) {
                    onEnd(slot);
                  }
                }}
                onPointerLeave={() => {
                  if (filled && holdLike) {
                    onEnd(slot);
                  }
                }}
              >
                <span className="w-full min-w-0 truncate text-[11px] font-bold leading-tight sm:text-xs">
                  {filled && label ? label : slot + 1}
                </span>
                {filled && sample?.duration_ms != null ? (
                  <span className="mt-0.5 max-w-full truncate text-[9px] tabular-nums opacity-75">
                    {formatDeckTimeTenth(sample.duration_ms)}
                  </span>
                ) : (
                  <span className="mt-0.5 text-[9px] uppercase opacity-75">sample</span>
                )}
              </DeckButton>
            </TrackDropZone>
          );
        })}
      </PadGridContainer>
    </>
  );
}
