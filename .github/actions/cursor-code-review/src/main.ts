import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { runReply } from './reply.js';
import { runReview } from './review.js';
import type { IntentResult } from './intent.js';
import { intentResultSchema } from './intent.js';

interface ActionInputs {
  mode: 'auto' | 'review' | 'reply';
  model: string;
  prNumber?: number;
  baseRef?: string;
  headSha?: string;
  workingDirectory: string;
  maxFiles: number;
  batchSize: number;
}

function env(name: string): string | undefined {
  const v = process.env[name];
  return v && v.trim() !== '' ? v : undefined;
}

function parseInputs(): ActionInputs {
  const modeRaw = (env('INPUT_MODE') ?? 'auto').toLowerCase();
  const mode =
    modeRaw === 'review' || modeRaw === 'reply' || modeRaw === 'auto'
      ? modeRaw
      : 'auto';
  const prNumber = env('INPUT_PR_NUMBER')
    ? Number(env('INPUT_PR_NUMBER'))
    : undefined;
  return {
    mode,
    model: env('INPUT_MODEL') ?? 'composer-2.5',
    prNumber:
      prNumber != null && Number.isInteger(prNumber) && prNumber > 0
        ? prNumber
        : undefined,
    baseRef: env('INPUT_BASE_REF'),
    headSha: env('INPUT_HEAD_SHA'),
    workingDirectory: resolve(env('INPUT_WORKING_DIRECTORY') ?? '.'),
    maxFiles: Number(env('INPUT_MAX_FILES') ?? 80),
    batchSize: Number(env('INPUT_REVIEW_COMMENT_BATCH_SIZE') ?? 50),
  };
}

function resolveRepo(): { owner: string; repo: string } {
  const full = env('GITHUB_REPOSITORY');
  if (full && full.includes('/')) {
    const [owner, repo] = full.split('/');
    return { owner: owner!, repo: repo! };
  }
  throw new Error('GITHUB_REPOSITORY is required');
}

function loadEvent(): Record<string, unknown> | null {
  const path = env('GITHUB_EVENT_PATH');
  if (!path || !existsSync(path)) return null;
  return JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>;
}

function decideMode(
  mode: ActionInputs['mode'],
  eventName: string | undefined,
  event: Record<string, unknown> | null,
): 'review' | 'reply' {
  if (mode === 'review' || mode === 'reply') return mode;
  if (eventName === 'pull_request_review_comment') {
    // Slash-command on a review comment → review; otherwise reply
    const comment = event?.comment as { body?: string } | undefined;
    const body = comment?.body ?? '';
    if (
      body.startsWith('/cursor-review') ||
      body.startsWith('@cursor-review')
    ) {
      return 'review';
    }
    return 'reply';
  }
  return 'review';
}

function resolvePrNumber(
  inputs: ActionInputs,
  event: Record<string, unknown> | null,
): number {
  if (inputs.prNumber) return inputs.prNumber;
  const pr = event?.pull_request as { number?: number } | undefined;
  if (pr?.number) return pr.number;
  const issue = event?.issue as {
    number?: number;
    pull_request?: unknown;
  } | undefined;
  if (issue?.pull_request && issue.number) return issue.number;
  throw new Error('Unable to resolve PR number');
}

function resolveCommentId(event: Record<string, unknown> | null): number {
  const comment = event?.comment as { id?: number } | undefined;
  if (comment?.id) return comment.id;
  throw new Error('Unable to resolve review comment id for reply mode');
}

function loadFixtureIntent(): IntentResult | undefined {
  const raw = env('CURSOR_REVIEW_FIXTURE_INTENT');
  if (!raw) return undefined;
  return intentResultSchema.parse(JSON.parse(raw));
}

async function main(): Promise<void> {
  const apiKey = env('CURSOR_API_KEY');
  if (!apiKey) {
    throw new Error('CURSOR_API_KEY is required');
  }
  const token = env('GITHUB_TOKEN');
  if (!token) {
    throw new Error('GITHUB_TOKEN is required');
  }

  const inputs = parseInputs();
  const eventName = env('GITHUB_EVENT_NAME');
  const event = loadEvent();
  const mode = decideMode(inputs.mode, eventName, event);
  const repo = resolveRepo();
  const prNumber = resolvePrNumber(inputs, event);
  const dryRun = env('CURSOR_REVIEW_DRY_RUN') === '1';

  console.log(
    JSON.stringify({
      mode,
      eventName,
      prNumber,
      model: inputs.model,
      cwd: inputs.workingDirectory,
      dryRun,
    }),
  );

  if (mode === 'reply') {
    const commentId = resolveCommentId(event);
    await runReply({
      token,
      apiKey,
      model: inputs.model,
      cwd: inputs.workingDirectory,
      repo,
      prNumber,
      commentId,
      headSha: inputs.headSha,
      dryRun,
      fixtureIntent: loadFixtureIntent(),
    });
  } else {
    await runReview({
      token,
      apiKey,
      model: inputs.model,
      cwd: inputs.workingDirectory,
      repo,
      prNumber,
      baseRef: inputs.baseRef,
      headSha: inputs.headSha,
      maxFiles: inputs.maxFiles,
      batchSize: inputs.batchSize,
      dryRun,
    });
  }
}

main().catch((err) => {
  console.error(err instanceof Error ? err.stack ?? err.message : err);
  process.exit(1);
});
