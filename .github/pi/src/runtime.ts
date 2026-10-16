import { readFileSync } from 'node:fs';
import { parseBatchSize, resolvePrNumber } from './config.ts';
import type { Env, EventPayload } from './types.ts';

export interface Runtime {
  token: string;
  owner: string;
  repo: string;
  prNumber: number;
  batchSize: number;
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

export function loadRuntime(env: Env = process.env): Runtime {
  const token = env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is required');
  const [owner, repo, ...rest] = (env.GITHUB_REPOSITORY ?? '').split('/');
  if (!owner || !repo || rest.length > 0) throw new Error('GITHUB_REPOSITORY must be owner/repo');
  const event = env.PI_REVIEW_PR_NUMBER ? {} : readEvent(env.GITHUB_EVENT_PATH);
  return {
    token,
    owner,
    repo,
    prNumber: resolvePrNumber(env, event),
    batchSize: parseBatchSize(env.PI_REVIEW_BATCH_SIZE),
    dryRun: env.PI_REVIEW_DRY_RUN === '1',
  };
}

export const repoPath = (rt: Runtime): string =>
  `/repos/${encodeURIComponent(rt.owner)}/${encodeURIComponent(rt.repo)}`;
