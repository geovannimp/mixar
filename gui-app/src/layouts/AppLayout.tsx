import { ToastProvider } from "@/components/ui/toast";
import { Outlet } from "react-router-dom";
import { AppHeader } from "../components/AppHeader";
import { WindowResizeBorder } from "../components/WindowResizeBorder";
import { EngineProvider, useEngine } from "../hooks/useEngine";

function AppLayoutContent() {
  const { status } = useEngine();

  return (
    <WindowResizeBorder className="flex flex-col bg-zinc-950 text-zinc-100">
      <AppHeader status={status} />
      <Outlet />
    </WindowResizeBorder>
  );
}

export function AppLayout() {
  return (
    <EngineProvider>
      <ToastProvider>
        <AppLayoutContent />
      </ToastProvider>
    </EngineProvider>
  );
}
