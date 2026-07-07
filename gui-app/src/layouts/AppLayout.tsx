import { Outlet } from "react-router-dom";
import { AppHeader } from "../components/AppHeader";
import { useEngine } from "../hooks/useEngine";

export function AppLayout() {
  const { status, busy, toggleEngine } = useEngine();

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
      <AppHeader status={status} busy={busy} onToggleEngine={toggleEngine} />
      <Outlet />
    </div>
  );
}
