import { FINDING_MARKER } from './constants.ts';
import { createGitHubClient } from './github.ts';
import { repoPath, type Runtime } from './runtime.ts';
import type { ReviewCommentPayload, ThreadMessage } from './types.ts';

export async function getReviewThread(rt: Runtime, commentId: number): Promise<string> {
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const comment = await api.gh<ReviewCommentPayload>('GET', `${base}/pulls/comments/${commentId}`);
  const rootId = comment.in_reply_to_id ?? comment.id;
  const all = await api.list<ReviewCommentPayload>(`${base}/pulls/${rt.prNumber}/comments`);
  const thread = all
    .filter((c) => c.id === rootId || c.in_reply_to_id === rootId)
    .sort((a, b) => a.created_at.localeCompare(b.created_at));
  const messages: ThreadMessage[] = thread.map((c) => ({
    id: c.id,
    author: c.user?.login ?? 'unknown',
    body: c.body ?? '',
    createdAt: c.created_at,
    isBot: c.user?.type === 'Bot',
  }));
  const root = messages.find((m) => m.id === rootId);
  if (!root) throw new Error(`root review comment ${rootId} not found on PR #${rt.prNumber}`);
  return JSON.stringify(
    {
      root,
      replies: messages.filter((m) => m.id !== rootId),
      nodeId: comment.node_id,
    },
    null,
    2,
  );
}

export async function replyToReviewComment(
  rt: Runtime,
  commentId: number,
  body: string,
): Promise<string> {
  const marked = body.includes(FINDING_MARKER) ? body : `${FINDING_MARKER}\n${body}`;
  if (rt.dryRun) return `dry run: would reply to comment ${commentId}:\n${marked}`;
  const api = createGitHubClient(rt.token);
  await api.gh(
    'POST',
    `${repoPath(rt)}/pulls/${rt.prNumber}/comments/${commentId}/replies`,
    { body: marked },
  );
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
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const comment = await api.gh<ReviewCommentPayload>('GET', `${base}/pulls/comments/${commentId}`);
  // Only threads this action opened may be resolved: the resolve tool is reachable
  // from a prompt-injected diff, and resolving a human's thread is a visible,
  // irreversible-looking action on someone else's review.
  const rootId = comment.in_reply_to_id ?? comment.id;
  const root =
    rootId === comment.id
      ? comment
      : await api.gh<ReviewCommentPayload>('GET', `${base}/pulls/comments/${rootId}`);
  if (!(root.body ?? '').includes(FINDING_MARKER)) {
    throw new Error(
      `refusing to resolve thread ${rootId}: root comment is not a pi-review finding (marker missing)`,
    );
  }
  const data = await api.graphql<{
    node: { pullRequestReviewThread: { id: string; isResolved: boolean } | null } | null;
  }>(THREAD_FOR_COMMENT_QUERY, { id: comment.node_id });
  const thread = data.node?.pullRequestReviewThread;
  if (!thread) throw new Error(`thread not found for comment ${commentId}`);
  if (thread.isResolved) return `already resolved (thread ${thread.id})`;
  if (rt.dryRun) return `dry run: would resolve thread ${thread.id}`;
  await api.graphql(RESOLVE_MUTATION, { id: thread.id });
  return `resolved thread ${thread.id}`;
}
