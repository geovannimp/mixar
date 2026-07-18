import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

export function isTauriApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getAppWindow(): Window {
  return getCurrentWindow();
}
