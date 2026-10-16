import type { ReviewConfig } from './rules.js';
import { isExcluded } from './rules.js';

export interface ChangedFile {
  path: string;
  status: string;
  patch?: string | null;
}

export interface SelectedFiles {
  selected: ChangedFile[];
  overflow: string[];
  skippedRemoved: string[];
  skippedExcluded: string[];
}

export function selectReviewFiles(
  files: ChangedFile[],
  config: ReviewConfig,
  maxFiles: number,
): SelectedFiles {
  const skippedRemoved: string[] = [];
  const skippedExcluded: string[] = [];
  const eligible: ChangedFile[] = [];

  for (const f of files) {
    if (f.status === 'removed') {
      skippedRemoved.push(f.path);
      continue;
    }
    if (isExcluded(f.path, config.exclude)) {
      skippedExcluded.push(f.path);
      continue;
    }
    eligible.push(f);
  }

  const selected = eligible.slice(0, maxFiles);
  const overflow = eligible.slice(maxFiles).map((f) => f.path);
  return { selected, overflow, skippedRemoved, skippedExcluded };
}

/** Bundle paths into groups of at most `bundleSize` when total > threshold. */
export function bundleFiles(
  files: ChangedFile[],
  opts: { threshold?: number; bundleSize?: number } = {},
): ChangedFile[][] {
  const threshold = opts.threshold ?? 40;
  const bundleSize = opts.bundleSize ?? 20;
  if (files.length <= threshold) return [files];
  const bundles: ChangedFile[][] = [];
  for (let i = 0; i < files.length; i += bundleSize) {
    bundles.push(files.slice(i, i + bundleSize));
  }
  return bundles;
}
