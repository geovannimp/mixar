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
      <div ref={setToastRoot} className="relative min-h-0 flex-1">
        <ToastProvider
          position="top-center"
          portalProps={toastRoot ? { container: toastRoot } : { container: null }}
        >
          <ControllerOfferBridge />
          <Outlet />
        </ToastProvider>
      </div>
    </WindowResizeBorder>
  );
}
