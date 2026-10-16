import { FINDING_MARKER, REVIEW_BODY_DONE, SUMMARY_ANCHOR } from './constants.ts';
import { chunk } from './config.ts';
import { parseDiffAnchors } from './diff.ts';
import { reviewableChangedFiles } from './files.ts';
import { buildReviewComments, buildSummaryBody, splitByConfidence } from './findings.ts';
import { GitHubHttpError, createGitHubClient } from './github.ts';
import { repoPath, type Runtime } from './runtime.ts';
import type { GitHubClient, PostReviewParams, PullPayload } from './types.ts';

export async function upsertStickySummary(
  api: GitHubClient,
  base: string,
  prNumber: number,
  body: string,
): Promise<void> {
  const comments = await api.list<{
    id: number;
    body?: string | null;
    user?: { type?: string; login?: string } | null;
  }>(`${base}/issues/${prNumber}/comments`);
  // Only reuse bot comments: a human quoting the anchor must not be overwritten.
  // If earlier runs left more than one, keep the oldest and delete the rest so the
  // thread does not accumulate stale summaries.
  const anchored = comments.filter(
    (c) => c.user?.type === 'Bot' && c.body?.includes(SUMMARY_ANCHOR),
  );
  const keep = anchored[0];
  if (keep) {
    await api.gh('PATCH', `${base}/issues/comments/${keep.id}`, { body });
    for (const dupe of anchored.slice(1)) {
      try {
        await api.gh('DELETE', `${base}/issues/comments/${dupe.id}`);
      } catch (err) {
        console.warn(
          `pi-review: could not delete duplicate summary comment ${dupe.id}: ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    }
  } else {
    await api.gh('POST', `${base}/issues/${prNumber}/comments`, { body });
  }
}

/** Statuses where re-posting the batch comment-by-comment can isolate the bad one. */
const DETERMINISTIC_BATCH_REJECTIONS = new Set([422]);

export async function postReview(rt: Runtime, params: PostReviewParams): Promise<string> {
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const { kept, dropped } = splitByConfidence(params.findings);
  const pr = await api.gh<PullPayload>('GET', `${base}/pulls/${rt.prNumber}`);
  const files = await reviewableChangedFiles(api, base, rt, pr.head.sha, pr.base.ref);
  const anchors = parseDiffAnchors(files.map((f) => ({ path: f.filename, patch: f.patch })));
  const { comments, outside } = buildReviewComments(kept, anchors);
  const summary = buildSummaryBody(params.summary_markdown, outside);

  if (rt.dryRun) {
    console.log(
      `pi-review post_review (dry run): comments=${comments.length} outside_diff=${outside.length} dropped_low_confidence=${dropped}`,
    );
    return JSON.stringify(
      { summary, comments, outside_diff: outside.length, dropped_low_confidence: dropped },
      null,
      2,
    );
  }

  await upsertStickySummary(api, base, rt.prNumber, summary);

  const commitId = pr.head.sha;
  const reviewPath = `${base}/pulls/${rt.prNumber}/reviews`;
  const errors: string[] = [];
  let posted = 0;

  // With no inline comments, submit no review at all: the sticky summary above is
  // upserted, so it already records that the run happened. A body-only review would
  // append another timeline entry per run.

  for (const [i, batch] of chunk(comments, rt.batchSize).entries()) {
    try {
      await api.gh('POST', reviewPath, {
        commit_id: commitId,
        event: 'COMMENT',
        body: i === 0 ? REVIEW_BODY_DONE : `${FINDING_MARKER}\n(continued review batch ${i + 1})`,
        comments: batch,
      });
      posted += batch.length;
    } catch (err) {
      const status = err instanceof GitHubHttpError ? err.status : 0;
      // Split the batch only for 422, the one status that means a comment in the
      // batch is itself invalid (usually a bad diff anchor) and would be dropped.
      // 429/403 (rate limits) and 408 are transient: fanning out to one request per
      // comment there would extend the abuse window instead of helping, and a lost
      // response may mean the batch already landed, so nothing is replayed.
      if (DETERMINISTIC_BATCH_REJECTIONS.has(status)) {
        console.log(
          `pi-review post_review: batch ${i + 1} rejected (${status}), retrying per comment: ${err instanceof Error ? err.message : String(err)}`,
        );
        for (const c of batch) {
          try {
            await api.gh('POST', reviewPath, {
              commit_id: commitId,
              event: 'COMMENT',
              body: `${FINDING_MARKER}\n(single-comment fallback)`,
              comments: [c],
            });
            posted += 1;
          } catch (inner) {
            errors.push(`${c.path}:${c.line}: ${inner instanceof Error ? inner.message : String(inner)}`);
          }
        }
      } else {
        errors.push(
          `batch ${i + 1} not replayed (status ${status || 'network'}): ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    }
  }

  const failed = errors.length;
  console.log(
    `pi-review post_review: posted=${posted} dropped_low_confidence=${dropped} outside_diff=${outside.length} failed=${failed}`,
  );
  return [
    `posted=${posted} dropped_low_confidence=${dropped} outside_diff=${outside.length} failed=${failed}`,
    ...errors,
  ].join('\n');
}
