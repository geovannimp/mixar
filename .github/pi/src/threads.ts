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
  const getComment = (id: number) =>
    octokit.rest.pulls.getReviewComment({ owner: rt.owner, repo: rt.repo, comment_id: id }).then(
      (r) => r.data,
    );

  const comment = await getComment(commentId);
  const rootId = comment.in_reply_to_id ?? comment.id;
  const root = rootId === comment.id ? comment : await getComment(rootId);

  // Authorisation is based on who *posted* the comment, not on its text. The marker
  // is a public literal, so anyone able to comment could paste it; only a thread this
  // token itself opened may be resolved.
  const { data: viewer } = await octokit.rest.users.getAuthenticated();
  if (root.user?.login !== viewer.login) {
    throw new Error(
      `refusing to resolve thread ${rootId}: root comment was authored by ${root.user?.login ?? 'unknown'}, not by ${viewer.login}`,
    );
  }
  // getReviewComment is scoped by repo + comment id only, so confirm the thread
  // belongs to the PR being reviewed.
  if (!root.pull_request_url.endsWith(`/${rt.prNumber}`)) {
    throw new Error(
      `refusing to resolve comment ${commentId}: it belongs to ${root.pull_request_url}, not to PR #${rt.prNumber}`,
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
