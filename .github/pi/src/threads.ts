import { FINDING_MARKER } from './constants.ts';
import { createOctokit } from './github.ts';
import type { Runtime } from './runtime.ts';

/**
 * PR number a review comment belongs to, read from its pull_request_url. The last
 * path segment is parsed and compared as a number: suffix-matching the string would
 * accept `.../pulls/123` for a review of PR 23, since that URL ends with "/23" too.
 */
function pullNumberFromUrl(url: string | undefined): number | null {
  const match = /\/(\d+)\/?$/.exec(url ?? '');
  return match ? Number(match[1]) : null;
}

/**
 * getReviewComment is scoped by repository and comment id only, so a comment id that
 * came from the model has to be checked against the PR under review before anything
 * acts on it - replying included, not just resolving. Exported so the unit test can
 * pin the PR-number comparison, which is what decides that.
 */
export function assertCommentOnPullRequest(
  rt: Runtime,
  comment: { pull_request_url?: string },
): void {
  const pullNumber = pullNumberFromUrl(comment.pull_request_url);
  if (pullNumber !== rt.prNumber) {
    throw new Error(
      `refusing to act on a comment from ${comment.pull_request_url ?? 'an unknown pull request'}: this run reviews PR #${rt.prNumber}`,
    );
  }
}

export async function replyToReviewComment(
  rt: Runtime,
  commentId: number,
  body: string,
): Promise<string> {
  const marked = body.includes(FINDING_MARKER) ? body : `${FINDING_MARKER}\n${body}`;
  const octokit = createOctokit(rt.token);
  // Replying is not restricted to threads this action opened - adding context to
  // someone else's thread is the point - but the comment still has to be on the PR
  // under review, and the id comes from the model.
  const { data: comment } = await octokit.rest.pulls.getReviewComment({
    owner: rt.owner,
    repo: rt.repo,
    comment_id: commentId,
  });
  assertCommentOnPullRequest(rt, comment);
  if (rt.dryRun) return `dry run: would reply to comment ${commentId}:\n${marked}`;
  await octokit.rest.pulls.createReplyForReviewComment({
    owner: rt.owner,
    repo: rt.repo,
    pull_number: rt.prNumber,
    comment_id: commentId,
    body: marked,
  });
  return 'replied';
}

const THREAD_FOR_COMMENT_QUERY = `query($id: ID!) {
  node(id: $id) {
    ... on PullRequestReviewComment {
      pullRequestReviewThread { id isResolved }
    }
  }
}`;

const RESOLVE_MUTATION = `mutation($id: ID!) {
  resolveReviewThread(input: { threadId: $id }) { thread { isResolved } }
}`;

export async function resolveReviewThread(rt: Runtime, commentId: number): Promise<string> {
  const octokit = createOctokit(rt.token);
  const getComment = (id: number) =>
    octokit.rest.pulls.getReviewComment({ owner: rt.owner, repo: rt.repo, comment_id: id }).then(
      (r) => r.data,
    );

  const comment = await getComment(commentId);
  assertCommentOnPullRequest(rt, comment);
  const rootId = comment.in_reply_to_id ?? comment.id;
  const root = rootId === comment.id ? comment : await getComment(rootId);
  assertCommentOnPullRequest(rt, root);

  // Authorisation is based on who *posted* the comment, not on its text. The marker
  // is a public literal, so anyone able to comment could paste it; only a thread this
  // token itself opened may be resolved.
  const { data: viewer } = await octokit.rest.users.getAuthenticated();
  if (root.user?.login !== viewer.login) {
    throw new Error(
      `refusing to resolve thread ${rootId}: root comment was authored by ${root.user?.login ?? 'unknown'}, not by ${viewer.login}`,
    );
  }

  const data = await octokit.graphql<{
    node: { pullRequestReviewThread: { id: string; isResolved: boolean } | null } | null;
  }>(THREAD_FOR_COMMENT_QUERY, { id: comment.node_id });
  const thread = data.node?.pullRequestReviewThread;
  if (!thread) throw new Error(`thread not found for comment ${commentId}`);
  if (thread.isResolved) return `already resolved (thread ${thread.id})`;
  if (rt.dryRun) return `dry run: would resolve thread ${thread.id}`;
  await octokit.graphql(RESOLVE_MUTATION, { id: thread.id });
  return `resolved thread ${thread.id}`;
}
