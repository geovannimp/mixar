import { FINDING_MARKER } from './constants.ts';
import { createOctokit } from './github.ts';
import type { Runtime } from './runtime.ts';

export async function replyToReviewComment(
  rt: Runtime,
  commentId: number,
  body: string,
): Promise<string> {
  const marked = body.includes(FINDING_MARKER) ? body : `${FINDING_MARKER}\n${body}`;
  if (rt.dryRun) return `dry run: would reply to comment ${commentId}:\n${marked}`;
  const octokit = createOctokit(rt.token);
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
  const { data: comment } = await octokit.rest.pulls.getReviewComment({
    owner: rt.owner,
    repo: rt.repo,
    comment_id: commentId,
  });
  // Only threads this action opened may be resolved: the resolve tool is reachable
  // from a prompt-injected diff, and resolving a human's thread is a visible,
  // irreversible-looking action on someone else's review. The marker is what
  // distinguishes our findings, and the workflow prompt requires it on every
  // inline finding body the model posts.
  const rootId = comment.in_reply_to_id ?? comment.id;
  const root =
    rootId === comment.id
      ? comment
      : (
          await octokit.rest.pulls.getReviewComment({
            owner: rt.owner,
            repo: rt.repo,
            comment_id: rootId,
          })
        ).data;
  if (!(root.body ?? '').includes(FINDING_MARKER)) {
    throw new Error(
      `refusing to resolve thread ${rootId}: root comment is not a pi-review finding (marker missing)`,
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
