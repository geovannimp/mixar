import { Octokit } from '@octokit/rest';
import { graphql } from '@octokit/graphql';
import {
  FINDING_MARKER,
  SUMMARY_ANCHOR,
  bodyHasOurMarker,
  formatFindingComment,
  formatSummaryBody,
} from './markers.js';
import type { ReviewFinding } from './findings.js';
import type { ChangedFile } from './files.js';

export interface RepoContext {
  owner: string;
  repo: string;
}

export interface PullInfo {
  number: number;
  title: string;
  baseRef: string;
  headSha: string;
  headRef: string;
}

export interface DiffAnchor {
  path: string;
  /** Set of absolute line numbers in the new file that appear in the diff. */
  lines: Set<number>;
}

export interface ReviewCommentInput {
  path: string;
  line: number;
  side: 'RIGHT';
  body: string;
  start_line?: number;
  start_side?: 'RIGHT';
}

export interface ThreadMessage {
  id: number;
  nodeId?: string;
  author: string;
  authorLogin: string;
  isBot: boolean;
  body: string;
  createdAt: string;
}

export function createOctokit(token: string): Octokit {
  return new Octokit({ auth: token });
}

export function createGraphql(token: string) {
  return graphql.defaults({
    headers: { authorization: `token ${token}` },
  });
}

export async function getPull(
  octokit: Octokit,
  repo: RepoContext,
  prNumber: number,
): Promise<PullInfo> {
  const { data } = await octokit.pulls.get({
    owner: repo.owner,
    repo: repo.repo,
    pull_number: prNumber,
  });
  return {
    number: data.number,
    title: data.title,
    baseRef: data.base.ref,
    headSha: data.head.sha,
    headRef: data.head.ref,
  };
}

export async function listChangedFiles(
  octokit: Octokit,
  repo: RepoContext,
  prNumber: number,
): Promise<ChangedFile[]> {
  const files: ChangedFile[] = [];
  for await (const response of octokit.paginate.iterator(
    octokit.pulls.listFiles,
    {
      owner: repo.owner,
      repo: repo.repo,
      pull_number: prNumber,
      per_page: 100,
    },
  )) {
    for (const f of response.data) {
      files.push({
        path: f.filename,
        status: f.status,
        patch: f.patch ?? null,
      });
    }
  }
  return files;
}

/** Parse unified-diff patches into new-file line anchors. */
export function getDiffAnchors(files: ChangedFile[]): Map<string, DiffAnchor> {
  const map = new Map<string, DiffAnchor>();
  for (const f of files) {
    const lines = new Set<number>();
    if (f.patch) {
      let newLine = 0;
      for (const raw of f.patch.split('\n')) {
        if (raw.startsWith('@@')) {
          const m = /\+(\d+)(?:,(\d+))?/.exec(raw);
          if (m) newLine = Number(m[1]);
          continue;
        }
        if (raw.startsWith('\\')) continue;
        if (raw.startsWith('-')) continue;
        if (raw.startsWith('+') || raw.startsWith(' ')) {
          if (newLine > 0) lines.add(newLine);
          newLine++;
        }
      }
    }
    map.set(f.path, { path: f.path, lines });
  }
  return map;
}

/** Clamp finding line to nearest diff anchor within ±tolerance; null if none. */
export function clampToDiffAnchor(
  anchors: Map<string, DiffAnchor>,
  path: string,
  line: number,
  tolerance = 3,
): number | null {
  const anchor = anchors.get(path);
  if (!anchor || anchor.lines.size === 0) return null;
  if (anchor.lines.has(line)) return line;
  let best: number | null = null;
  let bestDist = Infinity;
  for (const l of anchor.lines) {
    const d = Math.abs(l - line);
    if (d <= tolerance && d < bestDist) {
      best = l;
      bestDist = d;
    }
  }
  return best;
}

export async function createReviewBatched(
  octokit: Octokit,
  repo: RepoContext,
  opts: {
    prNumber: number;
    commitId: string;
    body: string;
    comments: ReviewCommentInput[];
    batchSize: number;
  },
): Promise<{ posted: number; dropped: number; errors: string[] }> {
  let posted = 0;
  let dropped = 0;
  const errors: string[] = [];
  const batches: ReviewCommentInput[][] = [];
  for (let i = 0; i < opts.comments.length; i += opts.batchSize) {
    batches.push(opts.comments.slice(i, i + opts.batchSize));
  }

  if (batches.length === 0) {
    await octokit.pulls.createReview({
      owner: repo.owner,
      repo: repo.repo,
      pull_number: opts.prNumber,
      commit_id: opts.commitId,
      event: 'COMMENT',
      body: opts.body,
    });
    return { posted: 0, dropped: 0, errors };
  }

  for (let bi = 0; bi < batches.length; bi++) {
    const batch = batches[bi]!;
    try {
      await octokit.pulls.createReview({
        owner: repo.owner,
        repo: repo.repo,
        pull_number: opts.prNumber,
        commit_id: opts.commitId,
        event: 'COMMENT',
        body: bi === 0 ? opts.body : `${FINDING_MARKER}\n(continued review batch ${bi + 1})`,
        comments: batch.map((c) => ({
          path: c.path,
          line: c.line,
          side: c.side,
          body: c.body,
          ...(c.start_line != null
            ? { start_line: c.start_line, start_side: c.start_side ?? 'RIGHT' }
            : {}),
        })),
      });
      posted += batch.length;
    } catch (err) {
      // Fail-open: drop line-mismatch comments one-by-one
      const msg = err instanceof Error ? err.message : String(err);
      errors.push(`batch ${bi}: ${msg}`);
      for (const c of batch) {
        try {
          await octokit.pulls.createReview({
            owner: repo.owner,
            repo: repo.repo,
            pull_number: opts.prNumber,
            commit_id: opts.commitId,
            event: 'COMMENT',
            body: `${FINDING_MARKER}\n(single-comment fallback)`,
            comments: [
              {
                path: c.path,
                line: c.line,
                side: c.side,
                body: c.body,
              },
            ],
          });
          posted += 1;
        } catch (inner) {
          dropped += 1;
          errors.push(
            `${c.path}:${c.line}: ${inner instanceof Error ? inner.message : String(inner)}`,
          );
        }
      }
    }
  }
  return { posted, dropped, errors };
}

export async function upsertStickySummary(
  octokit: Octokit,
  repo: RepoContext,
  prNumber: number,
  markdown: string,
): Promise<void> {
  const body = formatSummaryBody(markdown);
  const { data: comments } = await octokit.issues.listComments({
    owner: repo.owner,
    repo: repo.repo,
    issue_number: prNumber,
    per_page: 100,
  });
  const existing = comments.find((c) => c.body?.includes(SUMMARY_ANCHOR));
  if (existing) {
    await octokit.issues.updateComment({
      owner: repo.owner,
      repo: repo.repo,
      comment_id: existing.id,
      body,
    });
  } else {
    await octokit.issues.createComment({
      owner: repo.owner,
      repo: repo.repo,
      issue_number: prNumber,
      body,
    });
  }
}

export async function createReply(
  octokit: Octokit,
  repo: RepoContext,
  prNumber: number,
  commentId: number,
  body: string,
): Promise<void> {
  const marked = body.includes(FINDING_MARKER)
    ? body
    : `${FINDING_MARKER}\n${body}`;
  await octokit.pulls.createReplyForReviewComment({
    owner: repo.owner,
    repo: repo.repo,
    pull_number: prNumber,
    comment_id: commentId,
    body: marked,
  });
}

export function isOurReviewComment(body: string | null | undefined): boolean {
  return bodyHasOurMarker(body);
}

export async function getThreadContext(
  octokit: Octokit,
  repo: RepoContext,
  commentId: number,
): Promise<{
  root: ThreadMessage;
  replies: ThreadMessage[];
  inReplyToId: number | null;
}> {
  const { data: comment } = await octokit.pulls.getReviewComment({
    owner: repo.owner,
    repo: repo.repo,
    comment_id: commentId,
  });

  const rootId = comment.in_reply_to_id ?? comment.id;
  let root = comment;
  if (comment.in_reply_to_id) {
    const { data } = await octokit.pulls.getReviewComment({
      owner: repo.owner,
      repo: repo.repo,
      comment_id: comment.in_reply_to_id,
    });
    root = data;
  }

  // List review comments on the PR and filter by thread
  const all: ThreadMessage[] = [];
  for await (const page of octokit.paginate.iterator(
    octokit.pulls.listReviewComments,
    {
      owner: repo.owner,
      repo: repo.repo,
      pull_number: comment.pull_request_url
        ? Number(comment.pull_request_url.split('/').pop())
        : 0,
      per_page: 100,
    },
  )) {
    for (const c of page.data) {
      const belongs =
        c.id === rootId || c.in_reply_to_id === rootId;
      if (!belongs) continue;
      all.push({
        id: c.id,
        nodeId: c.node_id,
        author: c.user?.login ?? 'unknown',
        authorLogin: c.user?.login ?? 'unknown',
        isBot: c.user?.type === 'Bot',
        body: c.body ?? '',
        createdAt: c.created_at,
      });
    }
  }

  // Fallback if pull_number parse failed: use single-comment thread
  if (all.length === 0) {
    const toMsg = (c: typeof comment): ThreadMessage => ({
      id: c.id,
      nodeId: c.node_id,
      author: c.user?.login ?? 'unknown',
      authorLogin: c.user?.login ?? 'unknown',
      isBot: c.user?.type === 'Bot',
      body: c.body ?? '',
      createdAt: c.created_at,
    });
    return {
      root: toMsg(root),
      replies: comment.id === root.id ? [] : [toMsg(comment)],
      inReplyToId: comment.in_reply_to_id ?? null,
    };
  }

  all.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  const rootMsg = all.find((m) => m.id === rootId) ?? all[0]!;
  const replies = all.filter((m) => m.id !== rootMsg.id);
  return {
    root: rootMsg,
    replies,
    inReplyToId: comment.in_reply_to_id ?? null,
  };
}

export async function getThreadContextForPr(
  octokit: Octokit,
  repo: RepoContext,
  prNumber: number,
  commentId: number,
): Promise<{
  root: ThreadMessage;
  replies: ThreadMessage[];
  pullNumber: number;
}> {
  const { data: comment } = await octokit.pulls.getReviewComment({
    owner: repo.owner,
    repo: repo.repo,
    comment_id: commentId,
  });

  const rootId = comment.in_reply_to_id ?? comment.id;
  const all: ThreadMessage[] = [];
  for await (const page of octokit.paginate.iterator(
    octokit.pulls.listReviewComments,
    {
      owner: repo.owner,
      repo: repo.repo,
      pull_number: prNumber,
      per_page: 100,
    },
  )) {
    for (const c of page.data) {
      if (c.id === rootId || c.in_reply_to_id === rootId) {
        all.push({
          id: c.id,
          nodeId: c.node_id,
          author: c.user?.login ?? 'unknown',
          authorLogin: c.user?.login ?? 'unknown',
          isBot: c.user?.type === 'Bot',
          body: c.body ?? '',
          createdAt: c.created_at,
        });
      }
    }
  }
  all.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  const rootMsg = all.find((m) => m.id === rootId) ?? {
    id: comment.id,
    nodeId: comment.node_id,
    author: comment.user?.login ?? 'unknown',
    authorLogin: comment.user?.login ?? 'unknown',
    isBot: comment.user?.type === 'Bot',
    body: comment.body ?? '',
    createdAt: comment.created_at,
  };
  return {
    root: rootMsg,
    replies: all.filter((m) => m.id !== rootMsg.id),
    pullNumber: prNumber,
  };
}

export async function resolveThread(
  gql: ReturnType<typeof createGraphql>,
  threadNodeId: string,
): Promise<{ ok: boolean; error?: string }> {
  try {
    await gql(
      `mutation($id: ID!) {
        resolveReviewThread(input: { threadId: $id }) {
          thread { isResolved }
        }
      }`,
      { id: threadNodeId },
    );
    return { ok: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(`resolveReviewThread failed: ${msg}`);
    return { ok: false, error: msg };
  }
}

/** Resolve via GraphQL by looking up the review thread for a comment node id. */
export async function resolveThreadForComment(
  gql: ReturnType<typeof createGraphql>,
  repo: RepoContext,
  prNumber: number,
  commentNodeId: string,
): Promise<{ ok: boolean; error?: string }> {
  try {
    const data = await gql<{
      repository: {
        pullRequest: {
          reviewThreads: {
            nodes: Array<{
              id: string;
              isResolved: boolean;
              comments: { nodes: Array<{ id: string }> };
            }>;
          };
        };
      };
    }>(
      `query($owner: String!, $repo: String!, $number: Int!) {
        repository(owner: $owner, name: $repo) {
          pullRequest(number: $number) {
            reviewThreads(first: 100) {
              nodes {
                id
                isResolved
                comments(first: 50) {
                  nodes { id }
                }
              }
            }
          }
        }
      }`,
      { owner: repo.owner, repo: repo.repo, number: prNumber },
    );
    const thread = data.repository.pullRequest.reviewThreads.nodes.find((t) =>
      t.comments.nodes.some((c) => c.id === commentNodeId),
    );
    if (!thread) {
      return { ok: false, error: 'thread not found for comment' };
    }
    if (thread.isResolved) return { ok: true };
    return resolveThread(gql, thread.id);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.warn(`resolveThreadForComment failed: ${msg}`);
    return { ok: false, error: msg };
  }
}

export function findingsToReviewComments(
  findings: ReviewFinding[],
  anchors: Map<string, DiffAnchor>,
): { comments: ReviewCommentInput[]; outside: ReviewFinding[] } {
  const comments: ReviewCommentInput[] = [];
  const outside: ReviewFinding[] = [];
  for (const f of findings) {
    const end = f.end_line ?? f.start_line;
    const clamped = clampToDiffAnchor(anchors, f.path, end);
    if (clamped == null) {
      outside.push(f);
      continue;
    }
    let startLine = f.start_line;
    if (f.end_line != null && f.end_line !== f.start_line) {
      const startClamped = clampToDiffAnchor(anchors, f.path, f.start_line);
      startLine = startClamped ?? clamped;
    }
    const body = formatFindingComment({
      severity: f.severity,
      category: f.category,
      title: f.title,
      body: f.body,
    });
    const comment: ReviewCommentInput = {
      path: f.path,
      line: clamped,
      side: 'RIGHT',
      body,
    };
    if (startLine < clamped) {
      comment.start_line = startLine;
      comment.start_side = 'RIGHT';
    }
    comments.push(comment);
  }
  return { comments, outside };
}

export async function getAuthenticatedLogin(
  octokit: Octokit,
): Promise<string | null> {
  try {
    const { data } = await octokit.users.getAuthenticated();
    return data.login;
  } catch {
    // GITHUB_TOKEN may not allow user endpoint; fall back to github-actions[bot]
    return 'github-actions[bot]';
  }
}
