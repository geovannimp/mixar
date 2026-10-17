import { readFileSync } from 'node:fs';
import type { Env, EventPayload } from './types.ts';

export interface Runtime {
  token: string;
  owner: string;
  repo: string;
  prNumber: number;
  /**
   * Report what the extension would do without mutating anything. Reads still happen:
   * the ownership and authorship guards are part of what a dry run is meant to
   * exercise, and skipping them would make the report less trustworthy than the real
   * thing. Only the writes are skipped.
   */
  dryRun: boolean;
}

function readEvent(path: string | undefined): EventPayload {
  if (!path) return {};
  try {
    return JSON.parse(readFileSync(path, 'utf8')) as EventPayload;
  } catch (err) {
    throw new Error(
      `Failed to read GitHub event payload from ${path}: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}

/**
 * The PI_REVIEW_PR_NUMBER override, or null when it is unset. Single place that trims
 * and validates it, so `loadRuntime` (which must know whether to read the event
 * payload) and `resolvePrNumber` cannot drift apart.
 */
function prNumberOverride(env: Env): number | null {
  const raw = env.PI_REVIEW_PR_NUMBER?.trim();
  if (!raw) return null;
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0) {
    throw new Error(`PI_REVIEW_PR_NUMBER is not a valid PR number: ${raw}`);
  }
  return n;
}

export function resolvePrNumber(env: Env, event: EventPayload): number {
  const override = prNumberOverride(env);
  if (override !== null) return override;
  if (typeof event.pull_request?.number === 'number') return event.pull_request.number;
  if (typeof event.issue?.number === 'number') return event.issue.number;
  throw new Error(
    'cannot resolve PR number: set PI_REVIEW_PR_NUMBER or run on a pull_request/issue event',
  );
}

/**
 * Cached only for the real process environment: the runtime is fixed for the life of
 * a run, so tools do not need to re-validate the environment or re-read the event
 * payload on every call. Passing an explicit env (tests) bypasses the cache.
 */
let processRuntime: Runtime | undefined;

export function loadRuntime(env: Env = process.env): Runtime {
  if (env === process.env && processRuntime) return processRuntime;

  const token = env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is required');
  // Trim each segment: "owner/ " is non-empty, so a truthiness check alone would let
  // a padded value through into URL building.
  const [owner, repo, ...rest] = (env.GITHUB_REPOSITORY ?? '').split('/').map((s) => s.trim());
  if (!owner || !repo || rest.length > 0) throw new Error('GITHUB_REPOSITORY must be owner/repo');
  // Ask the same helper resolvePrNumber uses, or a whitespace-only override would skip
  // reading the event payload and then fail to resolve a PR that it carries.
  const event = prNumberOverride(env) !== null ? {} : readEvent(env.GITHUB_EVENT_PATH);
  const runtime: Runtime = {
    token,
    owner,
    repo,
    prNumber: resolvePrNumber(env, event),
    dryRun: env.PI_REVIEW_DRY_RUN === '1',
  };
  if (env === process.env) processRuntime = runtime;
  return runtime;
}
