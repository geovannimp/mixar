import { RotaryKnob } from "./RotaryKnob";
import { buttonIcon } from "../lib/ui";
import { cn } from "@/lib/utils";
import { useSettings } from "../hooks/useSettings";
import { engineActions, useCueMix, useMasterCue } from "../hooks/useEngine";

/** Compact Master Cue + Cue/Master mix for the app header. */
export function HeadphoneMonitorControls() {
  const cueMix = useCueMix();
  const masterCue = useMasterCue();
  const { settings } = useSettings();
  const previewEnabled = settings?.preview_enabled ?? false;
  const { setCueMix, setMasterCue } = engineActions;

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        disabled={!previewEnabled}
        aria-pressed={masterCue}
        aria-label="Master cue"
        title={
          previewEnabled
            ? "Master cue — route master mix to headphones"
            : "Enable Preview bus in Settings to use Master Cue"
        }
        className={cn(
          buttonIcon,
          "h-6 w-auto px-2 text-[9px] font-semibold uppercase tracking-wide",
          masterCue
            ? "border-amber-500/40 bg-amber-500/20 text-amber-300"
            : "border-white/10 text-zinc-500 hover:bg-zinc-800/90",
        )}
        onClick={() => {
          void setMasterCue(!masterCue);
        }}
      >
        Master Cue
      </button>
      <RotaryKnob
        label="Cue/Mst"
        value={cueMix}
        min={0}
        max={1}
        step={0.01}
        size="sm"
        disabled={!previewEnabled}
        ariaLabel="Cue master mix"
        accentClass="text-zinc-500"
        ringClass="border-amber-500/40"
        className="gap-0"
        formatValue={(value) => `${Math.round(value * 100)}%`}
        onValueChange={(mix) => {
          void setCueMix(mix);
        }}
      />
    </div>
  );
}
