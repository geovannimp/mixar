import { useState } from "react";
import { Outlet } from "react-router-dom";
import { ControllerOfferBridge } from "@/components/controller-offer-bridge";
import { ToastProvider } from "@/components/ui/toast";
import { AppHeader } from "@/components/shell/app-header";
import { WindowResizeBorder } from "@/components/shell/window-resize-border";

export function AppLayout() {
  const [toastRoot, setToastRoot] = useState<HTMLDivElement | null>(null);

  return (
    <WindowResizeBorder className="flex flex-col bg-zinc-950 text-zinc-100">
      <AppHeader />
      {/* flex-col so MixerPage/Settings `flex-1` fills height (Outlet parent must be a flex container). */}
      <div className="relative flex min-h-0 flex-1 flex-col">
        <ToastProvider
          position="top-center"
          portalProps={toastRoot ? { container: toastRoot } : { container: null }}
        >
          <div className="flex min-h-0 flex-1 flex-col">
            <ControllerOfferBridge />
            <Outlet />
          </div>
        </ToastProvider>
        {/* Overlay portal target: absolute so it does not steal flex space from the page. */}
        <div ref={setToastRoot} className="pointer-events-none absolute inset-0 z-60" aria-hidden />
      </div>
    </WindowResizeBorder>
  );
}
