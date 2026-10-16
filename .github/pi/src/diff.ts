import { minimatch } from 'minimatch';
import type { DiffAnchors } from './types.ts';

/**
 * Dotfiles match wildcards. `matchBase` makes a pattern without `/` also match
 * the basename, so `*.lock` matches `packages/app/yarn.lock`.
 */
export function matchesGlob(filePath: string, pattern: string): boolean {
  return minimatch(filePath, pattern, { dot: true, matchBase: true });
}

const HUNK_HEADER = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** Parse unified-diff patches into the new-file line numbers each path exposes to comments. */
export function parseDiffAnchors(
  files: Array<{ path: string; patch?: string | null }>,
): DiffAnchors {
  const anchors: DiffAnchors = new Map();
  for (const f of files) {
    const lines = new Set<number>();
    let newLine = 0;
    for (const raw of (f.patch ?? '').split('\n')) {
      const header = HUNK_HEADER.exec(raw);
      if (header) {
        newLine = Number(header[1]);
        continue;
      }
      // '-' lines and '\ No newline' markers do not occupy a new-file line.
      // newLine stays 0 until a @@ header is seen, so a malformed patch must not
      // contribute a 0 anchor (GitHub line numbers are 1-based).
      if (newLine > 0 && (raw.startsWith('+') || raw.startsWith(' '))) {
        lines.add(newLine);
        newLine++;
      }
    }
    anchors.set(f.path, lines);
  }
  return anchors;
}

/** Snap a line to the nearest diff anchor within `tolerance`; ties go to the first anchor found. */
export function clampToAnchor(
  anchors: DiffAnchors,
  path: string,
  line: number,
  tolerance = 3,
): number | null {
  const lines = anchors.get(path);
  if (!lines || lines.size === 0) return null;
  if (lines.has(line)) return line;
  let best: number | null = null;
  let bestDist = Infinity;
  for (const candidate of lines) {
    const dist = Math.abs(candidate - line);
    if (dist <= tolerance && dist < bestDist) {
      best = candidate;
      bestDist = dist;
    }
  }
  return best;
}

/** Every line in [start, end] is present in the diff (one contiguous run). */
export function isContiguous(anchors: Set<number>, start: number, end: number): boolean {
  for (let line = start; line <= end; line++) {
    if (!anchors.has(line)) return false;
  }
  return true;
}
