import { Outlet } from "react-router-dom";
import { ToastProvider } from "@/components/ui/toast";
import { AppHeader } from "@/components/AppHeader";
import { WindowResizeBorder } from "@/components/WindowResizeBorder";

function AppLayoutContent() {
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
