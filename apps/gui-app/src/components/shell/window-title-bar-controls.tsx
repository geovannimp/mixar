import { Minus, Square, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWindowControls } from "@/hooks/use-window-controls";

const controlButtonClass =
  "inline-flex h-full w-11 items-center justify-center text-zinc-400 transition hover:bg-white/8 hover:text-zinc-200";

export function WindowTitleBarControls() {
  const { isMaximized, toggleMaximize, minimize, close } = useWindowControls();
  return (
    <div className="flex items-stretch border-l border-white/8">
      <button
        type="button"
        className={controlButtonClass}
        aria-label="Minimize window"
        onClick={() => {
          minimize();
        }}
      >
        <Minus className="size-3.5" aria-hidden />
      </button>
      <button
        type="button"
        className={controlButtonClass}
        aria-label={isMaximized ? "Restore window" : "Maximize window"}
        onClick={() => {
          toggleMaximize();
        }}
      >
        <Square className={cn("size-3", isMaximized && "size-2.5")} aria-hidden />
      </button>
      <button
        type="button"
        className={cn(controlButtonClass, "hover:bg-red-500/20 hover:text-red-200")}
        aria-label="Close window"
        onClick={() => {
          close();
        }}
      >
        <X className="size-3.5" aria-hidden />
      </button>
    </div>
  );
}
