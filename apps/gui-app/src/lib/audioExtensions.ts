import { invoke } from "@tauri-apps/api/core";
import { isTauriApp } from "./tauriApp";

export const FALLBACK_AUDIO_EXTENSIONS = [
  "mp3",
  "flac",
  "wav",
  "aiff",
  "aif",
  "ogg",
  "m4a",
  "aac",
  "opus",
  "wma",
  "alac",
] as const;

let cachedExtensions: string[] | null = null;

export async function getSupportedAudioExtensions(): Promise<string[]> {
  if (cachedExtensions) {
    return cachedExtensions;
  }

  if (!isTauriApp()) {
    cachedExtensions = [...FALLBACK_AUDIO_EXTENSIONS];
    return cachedExtensions;
  }

  cachedExtensions = await invoke<string[]>("get_supported_audio_extensions");
  return cachedExtensions;
}
