import type { MouseEvent, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { getAppWindow, isTauriApp } from "../lib/tauriApp";

interface TitleBarDragRegionProps {
  children?: ReactNode;
  className?: string;
}

export function TitleBarDragRegion({ children, className }: TitleBarDragRegionProps) {
  const handleMouseDown = (event: MouseEvent<HTMLDivElement>) => {
    if (!isTauriApp() || event.buttons !== 1) {
      return;
    }

    const appWindow = getAppWindow();
    if (event.detail === 2) {
      void appWindow.toggleMaximize();
      return;
    }

    void appWindow.startDragging();
  };

  return (
    <div data-tauri-drag-region className={cn(className)} onMouseDown={handleMouseDown}>
      {children}
    </div>
  );
}
