import { readFileSync } from 'node:fs';
import type { Env, EventPayload } from './types.ts';

export interface Runtime {
  token: string;
  owner: string;
  repo: string;
  prNumber: number;
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

export function loadRuntime(env: Env = process.env): Runtime {
  const token = env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is required');
  const [owner, repo, ...rest] = (env.GITHUB_REPOSITORY ?? '').split('/');
  if (!owner || !repo || rest.length > 0) throw new Error('GITHUB_REPOSITORY must be owner/repo');
  // Trim here exactly as resolvePrNumber does, or a whitespace-only override would
  // skip reading the event payload and then fail to resolve a PR that it carries.
  const override = env.PI_REVIEW_PR_NUMBER?.trim();
  const event = override ? {} : readEvent(env.GITHUB_EVENT_PATH);
  return {
    token,
    owner,
    repo,
    prNumber: resolvePrNumber(env, event),
    dryRun: env.PI_REVIEW_DRY_RUN === '1',
  };
}
