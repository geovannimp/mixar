import type { KeyDisplayMode } from "../types";

const MAJOR_KEYS = ["C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#", "F"] as const;

const MINOR_KEYS = [
  "Am",
  "Em",
  "Bm",
  "F#m",
  "C#m",
  "G#m",
  "D#m",
  "A#m",
  "Fm",
  "Cm",
  "Gm",
  "Dm",
] as const;

const KEY_DISPLAY_MODE_STORAGE = "dj-key-display-mode";

export function getKeyDisplayMode(): KeyDisplayMode {
  const stored = localStorage.getItem(KEY_DISPLAY_MODE_STORAGE);
  return stored === "camelot" ? "camelot" : "musical";
}

export function setKeyDisplayMode(mode: KeyDisplayMode): void {
  localStorage.setItem(KEY_DISPLAY_MODE_STORAGE, mode);
}

function musicalToCamelot(key: string): string | null {
  const trimmed = key.trim();
  const majorIndex = MAJOR_KEYS.indexOf(trimmed as (typeof MAJOR_KEYS)[number]);
  if (majorIndex >= 0) {
    return `${majorIndex + 1}A`;
  }
  const minorIndex = MINOR_KEYS.indexOf(trimmed as (typeof MINOR_KEYS)[number]);
  if (minorIndex >= 0) {
    return `${minorIndex + 1}B`;
  }
  return null;
}

function camelotToMusical(code: string): string | null {
  const trimmed = code.trim().toUpperCase();
  const minor = trimmed.endsWith("B");
  const major = trimmed.endsWith("A");
  if (!minor && !major) {
    return null;
  }
  const numberText = trimmed.slice(0, -1);
  const number = Number.parseInt(numberText, 10);
  if (!Number.isFinite(number) || number < 1 || number > 12) {
    return null;
  }
  const index = number - 1;
  if (minor) {
    return MINOR_KEYS[index] ?? null;
  }
  return MAJOR_KEYS[index] ?? null;
}

export function formatDeckKey(key: string | null | undefined, mode: KeyDisplayMode): string {
  if (!key?.trim()) {
    return "—";
  }
  const trimmed = key.trim();
  if (mode === "musical") {
    return camelotToMusical(trimmed) ?? trimmed;
  }
  return musicalToCamelot(trimmed) ?? trimmed;
}
