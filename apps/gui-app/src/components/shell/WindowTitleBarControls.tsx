import { useEffect, useState } from "react";
import { Minus, Square, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { getAppWindow } from "@/lib/tauriApp";

const controlButtonClass =
  "inline-flex h-full w-11 items-center justify-center text-zinc-400 transition hover:bg-white/8 hover:text-zinc-200";

export function WindowTitleBarControls() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const appWindow = getAppWindow();
    let disposed = false;

    const syncMaximized = async () => {
      const maximized = await appWindow.isMaximized();
      if (!disposed) {
        setIsMaximized(maximized);
      }
    };

    void syncMaximized();

    const unlistenPromise = appWindow.onResized(() => {
      void syncMaximized();
    });

    return () => {
      disposed = true;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <div className="flex h-full shrink-0 items-stretch border-l border-white/8">
      <button
        type="button"
        className={controlButtonClass}
        aria-label="Minimize window"
        onClick={() => {
          void getAppWindow().minimize();
        }}
      >
        <Minus className="size-3.5" aria-hidden />
      </button>
      <button
        type="button"
        className={controlButtonClass}
        aria-label={isMaximized ? "Restore window" : "Maximize window"}
        onClick={() => {
          void getAppWindow().toggleMaximize();
        }}
      >
        <Square className={cn("size-3", isMaximized && "size-2.5")} aria-hidden />
      </button>
      <button
        type="button"
        className={`${controlButtonClass} hover:bg-red-500/20 hover:text-red-200`}
        aria-label="Close window"
        onClick={() => {
          void getAppWindow().close();
        }}
      >
        <X className="size-3.5" aria-hidden />
      </button>
    </div>
  );
}
