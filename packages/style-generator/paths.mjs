// @ts-check
// One place that names the token documents and the repo root, so the emitters,
// the Style Dictionary config and their generated headers cannot disagree.

import { fileURLToPath } from 'node:url';

/** Repo root, anchored to this package rather than the working directory. */
export const repoRoot = new URL('../../', import.meta.url);

/** Repo-relative path → absolute. */
export const at = (/** @type {string} */ path) =>
  fileURLToPath(new URL(path, repoRoot));

export const PRIMITIVES = 'tokens/primitives.json';
export const SEMANTIC = 'tokens/semantic.json';
export const WEB = 'tokens/web.json';
