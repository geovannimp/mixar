import { NavLink } from "react-router-dom";
import { useEngineHeaderInfo } from "@/hooks/engine/useEngineHeaderInfo";
import { isTauriApp } from "@/lib/tauriApp";
import { statusPillClass } from "@/lib/ui";
import { HeadphoneMonitorControls } from "@/components/mixer/HeadphoneMonitorControls";
import { StatusPill } from "./StatusPill";
import { TitleBarDragRegion } from "./TitleBarDragRegion";
import { WindowTitleBarControls } from "./WindowTitleBarControls";
import {
  Popover,
  PopoverDescription,
  PopoverPopup,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";

function navClass({ isActive }: { isActive: boolean }): string {
  return isActive
    ? "rounded border border-white/20 bg-white/10 px-2.5 py-1 text-xs font-semibold uppercase tracking-wide text-zinc-100"
    : "rounded border border-transparent px-2.5 py-1 text-xs font-semibold uppercase tracking-wide text-zinc-500 hover:text-zinc-300";
}

export function AppHeader() {
  const { running, backend, sampleRate } = useEngineHeaderInfo();
  const showWindowControls = isTauriApp();

  return (
    <header className="relative z-40 flex h-12 shrink-0 items-stretch border-b border-white/8 bg-zinc-900/80 backdrop-blur-sm">
      <div className="flex min-w-0 items-center gap-3 px-4">
        <TitleBarDragRegion className="flex shrink-0 items-center">
          <h1 className="text-sm font-bold uppercase tracking-widest text-zinc-200">Rust DJ</h1>
        </TitleBarDragRegion>
        <nav
          className="flex items-center gap-1"
          onMouseDown={(event) => {
            event.stopPropagation();
          }}
        >
          <NavLink to="/" end className={navClass}>
            Decks
          </NavLink>
          <NavLink to="/settings" className={navClass}>
            Settings
          </NavLink>
        </nav>
      </div>

      <TitleBarDragRegion className="min-w-6 flex-1" />

      <div className="flex shrink-0 items-center gap-2.5 px-2 sm:gap-3 sm:px-3">
        <HeadphoneMonitorControls />
        {running ? (
          <Popover>
            <PopoverTrigger aria-label="Engine status details" className={statusPillClass(true)}>
              Running
            </PopoverTrigger>
            <PopoverPopup align="end" side="bottom" sideOffset={8} className="w-56">
              <PopoverTitle className="text-sm">Engine</PopoverTitle>
              <PopoverDescription className="mt-1.5 space-y-1 text-xs text-zinc-300">
                <div className="flex justify-between gap-3">
                  <span className="text-zinc-500">Backend</span>
                  <span className="font-medium text-zinc-200">{backend}</span>
                </div>
                <div className="flex justify-between gap-3">
                  <span className="text-zinc-500">Sample rate</span>
                  <span className="font-medium text-zinc-200">{sampleRate} Hz</span>
                </div>
              </PopoverDescription>
            </PopoverPopup>
          </Popover>
        ) : (
          <StatusPill active={false}>Stopped</StatusPill>
        )}
      </div>

      {showWindowControls && <WindowTitleBarControls />}
    </header>
  );
}
