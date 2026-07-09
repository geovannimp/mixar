import { buttonCompact } from "../lib/ui";
import type { DeckStatus } from "../types";

interface DeckPerformancePanelProps {
  deck: DeckStatus;
  disabled?: boolean;
  onToggleQuantize: (enabled: boolean) => void;
}

export function DeckPerformancePanel({
  deck,
  disabled,
  onToggleQuantize,
}: DeckPerformancePanelProps) {
  return (
    <div className="flex shrink-0 items-center justify-end gap-1">
      <button
        type="button"
        disabled={disabled}
        className={`${buttonCompact} border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${
          deck.quantize
            ? "border-emerald-500/50 bg-emerald-500/15 text-emerald-200"
            : "border-white/10 bg-black/25 text-zinc-400"
        }`}
        onClick={() => onToggleQuantize(!deck.quantize)}
      >
        Q
      </button>
    </div>
  );
}
