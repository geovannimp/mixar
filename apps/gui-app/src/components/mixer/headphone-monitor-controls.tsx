import { RotaryKnob } from "./rotary-knob";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useSettings } from "@/hooks/use-settings";
import { useCueMix } from "@/hooks/engine/use-cue-mix";
import { useMasterCue } from "@/hooks/engine/use-master-cue";
import { engineActions } from "@/stores/engine-store";

/** Compact Master Cue + Cue/Master mix for the app header. */
export function HeadphoneMonitorControls() {
  const cueMix = useCueMix();
  const masterCue = useMasterCue();
  const { settings } = useSettings();
  const previewEnabled = settings?.preview_enabled ?? false;
  const { setCueMix, setMasterCue } = engineActions;

  return (
    <div className="flex items-center gap-2">
      <Button
        type="button"
        size="xs"
        variant="outline"
        disabled={!previewEnabled}
        aria-pressed={masterCue}
        aria-label="Master cue"
        title={
          previewEnabled
            ? "Master cue — route master mix to headphones"
            : "Enable Preview bus in Settings to use Master Cue"
        }
        className={cn(
          "w-auto px-2 text-[9px] font-semibold uppercase tracking-wide",
          masterCue
            ? "border-amber-500/40 bg-amber-500/20 text-amber-300 hover:bg-amber-500/25 hover:text-amber-300"
            : "border-white/10 text-zinc-500 hover:bg-zinc-800/90",
        )}
        onClick={() => {
          void setMasterCue(!masterCue);
        }}
      >
        Master Cue
      </Button>
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
        onValueChange={(mix) => {
          void setCueMix(mix);
        }}
      />
    </div>
  );
}
