import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

export type AppEnvironment = "TAURI" | "WEB";

export const APP_ENVIRONMENT: AppEnvironment =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window ? "TAURI" : "WEB";

export function isTauriApp(): boolean {
  return APP_ENVIRONMENT === "TAURI";
}

export function getAppWindow(): Window {
  return getCurrentWindow();
}
