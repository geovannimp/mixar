import { Outlet } from "react-router-dom";
import { ToastProvider } from "@/components/ui/toast";
import { AppHeader } from "@/components/shell/app-header";
import { WindowResizeBorder } from "@/components/shell/window-resize-border";

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
