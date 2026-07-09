import { ToastProvider } from "@/components/ui/toast";
import { Outlet } from "react-router-dom";
import { AppHeader } from "../components/AppHeader";
import { WindowResizeBorder } from "../components/WindowResizeBorder";
import { useEngineBootstrap } from "../hooks/useEngineBootstrap";

function AppLayoutContent() {
  useEngineBootstrap();

  return (
    <WindowResizeBorder className="flex flex-col bg-zinc-950 text-zinc-100">
      <AppHeader />
      <Outlet />
    </WindowResizeBorder>
  );
}

export function AppLayout() {
  return (
    <ToastProvider>
      <AppLayoutContent />
    </ToastProvider>
  );
}
