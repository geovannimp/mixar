import type { KeyDisplayMode } from "@/types";

/** Circle-of-fifths majors starting at C. Index `i` → Camelot `(i + 7) % 12 + 1` + `B`. */
const MAJOR_KEYS = ["C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#", "F"] as const;

/** Relative minors starting at Am. Index `i` → Camelot `(i + 7) % 12 + 1` + `A`. */
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

/** Mixed In Key: C→8B, Am→8A (A=minor, B=major). */
const CAMELOT_OFFSET = 7;

function musicalToCamelot(key: string): string | null {
  const trimmed = key.trim();
  const majorIndex = MAJOR_KEYS.indexOf(trimmed as (typeof MAJOR_KEYS)[number]);
  if (majorIndex >= 0) {
    return `${((majorIndex + CAMELOT_OFFSET) % 12) + 1}B`;
  }
  const minorIndex = MINOR_KEYS.indexOf(trimmed as (typeof MINOR_KEYS)[number]);
  if (minorIndex >= 0) {
    return `${((minorIndex + CAMELOT_OFFSET) % 12) + 1}A`;
  }
  return null;
}

function camelotToMusical(code: string): string | null {
  const trimmed = code.trim();
  if (trimmed.length < 2) {
    return null;
  }
  const upper = trimmed.toUpperCase();
  const minor = upper.endsWith("A");
  const major = upper.endsWith("B");
  if (!minor && !major) {
    return null;
  }
  const numberText = upper.slice(0, -1);
  const number = Number.parseInt(numberText, 10);
  if (!Number.isFinite(number) || number < 1 || number > 12) {
    return null;
  }
  const index = (number + 12 - 1 - CAMELOT_OFFSET) % 12;
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
