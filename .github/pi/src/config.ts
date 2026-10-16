import { DEFAULT_BATCH_SIZE } from './constants.ts';
import type { Env, EventPayload, PathRule, ReviewConfig } from './types.ts';

export function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

/**
 * GitHub rejects oversized review payloads, and a batch too large also makes the
 * 422 fallback replay everything at once, so cap it.
 */
export const MAX_BATCH_SIZE = 100;

export function parseBatchSize(raw: string | undefined): number {
  const n = Number.parseInt(raw ?? '', 10);
  if (!Number.isInteger(n) || n < 1) return DEFAULT_BATCH_SIZE;
  if (n > MAX_BATCH_SIZE) {
    console.warn(`pi-review: PI_REVIEW_BATCH_SIZE=${n} exceeds ${MAX_BATCH_SIZE}; clamping`);
    return MAX_BATCH_SIZE;
  }
  return n;
}

/**
 * Parse the base-branch config. Null (absent file) yields empty lists. Malformed JSON
 * throws, so a broken base config fails the review visibly instead of silently reviewing
 * excluded files.
 */
export function parseReviewConfig(raw: string | null): ReviewConfig {
  if (raw == null) return { exclude: [], rules: [] };
  const obj = (JSON.parse(raw) ?? {}) as { exclude?: unknown; rules?: unknown };
  const exclude = Array.isArray(obj.exclude)
    ? obj.exclude.filter((p): p is string => typeof p === 'string')
    : [];
  const rules = Array.isArray(obj.rules)
    ? obj.rules
        .filter(
          (r): r is PathRule =>
            typeof r === 'object' &&
            r !== null &&
            typeof r.path === 'string' &&
            typeof r.rule === 'string',
        )
        .map((r) => ({ path: r.path, rule: r.rule }))
    : [];
  return { exclude, rules };
}

export function resolvePrNumber(env: Env, event: EventPayload): number {
  const override = env.PI_REVIEW_PR_NUMBER?.trim();
  if (override) {
    const n = Number(override);
    if (Number.isInteger(n) && n > 0) return n;
    throw new Error(`PI_REVIEW_PR_NUMBER is not a valid PR number: ${override}`);
  }
  if (typeof event.pull_request?.number === 'number') return event.pull_request.number;
  if (typeof event.issue?.number === 'number') return event.issue.number;
  throw new Error(
    'cannot resolve PR number: set PI_REVIEW_PR_NUMBER or run on a pull_request/issue event',
  );
}
