import { FINDING_MARKER, REVIEW_BODY_DONE, SUMMARY_ANCHOR } from './constants.ts';
import { chunk } from './config.ts';
import { parseDiffAnchors } from './diff.ts';
import { reviewableChangedFiles } from './files.ts';
import { buildReviewComments, buildSummaryBody, splitByConfidence } from './findings.ts';
import { createOctokit, httpStatus, type GitHubOctokit } from './github.ts';
import type { Runtime } from './runtime.ts';
import type { PostReviewParams } from './types.ts';

export async function upsertStickySummary(
  octokit: GitHubOctokit,
  rt: Runtime,
  body: string,
): Promise<void> {
  const comments = await octokit.paginate('GET /repos/{owner}/{repo}/issues/{issue_number}/comments', {
    owner: rt.owner,
    repo: rt.repo,
    issue_number: rt.prNumber,
    per_page: 100,
  });
  // Only reuse bot comments: a human quoting the anchor must not be overwritten.
  // If earlier runs left more than one, keep the oldest and delete the rest so the
  // thread does not accumulate stale summaries.
  const anchored = comments.filter(
    (c) => c.user?.type === 'Bot' && c.body?.includes(SUMMARY_ANCHOR),
  );
  const keep = anchored[0];
  if (keep) {
    await octokit.request('PATCH /repos/{owner}/{repo}/issues/comments/{comment_id}', {
      owner: rt.owner,
      repo: rt.repo,
      comment_id: keep.id,
      body,
    });
    for (const dupe of anchored.slice(1)) {
      try {
        await octokit.request('DELETE /repos/{owner}/{repo}/issues/comments/{comment_id}', {
          owner: rt.owner,
          repo: rt.repo,
          comment_id: dupe.id,
        });
      } catch (err) {
        console.warn(
          `pi-review: could not delete duplicate summary comment ${dupe.id}: ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    }
  } else {
    await octokit.request('POST /repos/{owner}/{repo}/issues/{issue_number}/comments', {
      owner: rt.owner,
      repo: rt.repo,
      issue_number: rt.prNumber,
      body,
    });
  }
}

/** Statuses where re-posting the batch comment-by-comment can isolate the bad one. */
const DETERMINISTIC_BATCH_REJECTIONS = new Set([422]);

export async function postReview(rt: Runtime, params: PostReviewParams): Promise<string> {
  const octokit = createOctokit(rt.token);
  const { kept, dropped } = splitByConfidence(params.findings);
  const { data: pr } = await octokit.request('GET /repos/{owner}/{repo}/pulls/{pull_number}', {
    owner: rt.owner,
    repo: rt.repo,
    pull_number: rt.prNumber,
  });
  const files = await reviewableChangedFiles(octokit, rt, pr.head.sha, pr.base.ref);
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

  await upsertStickySummary(octokit, rt, summary);

  const commitId = pr.head.sha;
  const errors: string[] = [];
  let posted = 0;

  // With no inline comments, submit no review at all: the sticky summary above is
  // upserted, so it already records that the run happened. A body-only review would
  // append another timeline entry per run.

  const reviewParams = {
    owner: rt.owner,
    repo: rt.repo,
    pull_number: rt.prNumber,
    commit_id: commitId,
    event: 'COMMENT' as const,
  };

  for (const [i, batch] of chunk(comments, rt.batchSize).entries()) {
    try {
      await octokit.request('POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews', {
        ...reviewParams,
        body: i === 0 ? REVIEW_BODY_DONE : `${FINDING_MARKER}\n(continued review batch ${i + 1})`,
        comments: batch,
      });
      posted += batch.length;
    } catch (err) {
      const status = httpStatus(err);
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
            await octokit.request('POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews', {
              ...reviewParams,
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
  if (failed > 0) {
    // The summary was upserted before the batches, so amend it: a tool result the
    // model may skim is not enough to tell a human that findings went missing.
    // Re-posting is deliberately not attempted — a failed batch may have landed.
    try {
      await upsertStickySummary(
        octokit,
        rt,
        `${summary}\n\n> ⚠️ ${failed} item(s) could not be posted to this pull request. See the workflow log for details.`,
      );
    } catch (err) {
      console.warn(
        `pi-review: could not amend the summary with the post failure: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }
  const headline = `posted=${posted} dropped_low_confidence=${dropped} outside_diff=${outside.length} failed=${failed}`;
  return [
    failed > 0
      ? `${headline}\nWARNING: ${failed} item(s) were NOT posted to the pull request (details below). Mention this in your final message.`
      : headline,
    ...errors,
  ].join('\n');
}
