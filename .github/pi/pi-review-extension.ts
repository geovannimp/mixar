import { readFileSync } from 'node:fs';
import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { minimatch } from 'minimatch';
import { Octokit } from '@octokit/rest';
import { graphql as gql } from '@octokit/graphql';
import { retry } from '@octokit/plugin-retry';

export const FINDING_MARKER = '<!-- pi-code-review -->';
export const SUMMARY_ANCHOR = '<!-- pi-code-review-summary -->';
export const CONFIDENCE_FLOOR = 0.55;
export const DEFAULT_BATCH_SIZE = 50;
export const REVIEW_CONFIG_PATH = '.pi-review/config.json';
export const SEVERITIES = ['critical', 'high', 'medium', 'low'] as const;
export const CATEGORIES = [
  'bug',
  'security',
  'performance',
  'maintainability',
  'test',
  'style',
  'documentation',
  'other',
] as const;

const REVIEW_BODY_DONE = `${FINDING_MARKER}\nReview posted; see summary comment.`;

export type Severity = (typeof SEVERITIES)[number];
export type Category = (typeof CATEGORIES)[number];

export interface ReviewFinding {
  path: string;
  start_line: number;
  end_line?: number;
  severity: Severity;
  category: Category;
  title: string;
  body: string;
  confidence: number;
}

export interface PathRule {
  path: string;
  rule: string;
}

export interface ReviewConfig {
  exclude: string[];
  rules: PathRule[];
}

export interface ReviewCommentInput {
  path: string;
  line: number;
  side: 'RIGHT';
  body: string;
  start_line?: number;
  start_side?: 'RIGHT';
}

/** Map of changed path to the set of new-file line numbers that appear in the diff. */
export type DiffAnchors = Map<string, Set<number>>;

export interface ThreadMessage {
  id: number;
  author: string;
  body: string;
  createdAt: string;
  isBot: boolean;
}

export interface PostReviewParams {
  summary_markdown: string;
  findings: ReviewFinding[];
}

export interface GitHubClient {
  gh<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
  list<T>(path: string): Promise<T[]>;
  graphql<T>(query: string, variables: Record<string, unknown>): Promise<T>;
}

type EventPayload = {
  pull_request?: { number?: number };
  issue?: { number?: number };
};

type Env = Record<string, string | undefined>;

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/**
 * Dotfiles match wildcards. `matchBase` makes a pattern without `/` also match
 * the basename, so `*.lock` matches `packages/app/yarn.lock`.
 */
export function matchesGlob(filePath: string, pattern: string): boolean {
  return minimatch(filePath, pattern, { dot: true, matchBase: true });
}

const HUNK_HEADER = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** Parse unified-diff patches into the new-file line numbers each path exposes to comments. */
export function parseDiffAnchors(
  files: Array<{ path: string; patch?: string | null }>,
): DiffAnchors {
  const anchors: DiffAnchors = new Map();
  for (const f of files) {
    const lines = new Set<number>();
    let newLine = 0;
    for (const raw of (f.patch ?? '').split('\n')) {
      const header = HUNK_HEADER.exec(raw);
      if (header) {
        newLine = Number(header[1]);
        continue;
      }
      // '-' lines and '\ No newline' markers do not occupy a new-file line.
      // newLine stays 0 until a @@ header is seen, so a malformed patch must not
      // contribute a 0 anchor (GitHub line numbers are 1-based).
      if (newLine > 0 && (raw.startsWith('+') || raw.startsWith(' '))) {
        lines.add(newLine);
        newLine++;
      }
    }
    anchors.set(f.path, lines);
  }
  return anchors;
}

/** Snap a line to the nearest diff anchor within `tolerance`; ties go to the first anchor found. */
export function clampToAnchor(
  anchors: DiffAnchors,
  path: string,
  line: number,
  tolerance = 3,
): number | null {
  const lines = anchors.get(path);
  if (!lines || lines.size === 0) return null;
  if (lines.has(line)) return line;
  let best: number | null = null;
  let bestDist = Infinity;
  for (const candidate of lines) {
    const dist = Math.abs(candidate - line);
    if (dist <= tolerance && dist < bestDist) {
      best = candidate;
      bestDist = dist;
    }
  }
  return best;
}

export function splitByConfidence(
  findings: ReviewFinding[],
  floor = CONFIDENCE_FLOOR,
): { kept: ReviewFinding[]; dropped: number } {
  const kept = findings.filter((f) => f.confidence >= floor);
  return { kept, dropped: findings.length - kept.length };
}

export function formatFindingBody(
  f: Pick<ReviewFinding, 'severity' | 'category' | 'title' | 'body'>,
): string {
  return `${FINDING_MARKER}\n**[${f.severity}/${f.category}] ${f.title}**\n\n${f.body.trim()}`;
}

/**
 * Turn findings into inline comments anchored to the diff. Findings whose end line
 * cannot be clamped onto the diff are returned as `outside`.
 */
export function buildReviewComments(
  findings: ReviewFinding[],
  anchors: DiffAnchors,
): { comments: ReviewCommentInput[]; outside: ReviewFinding[] } {
  const comments: ReviewCommentInput[] = [];
  const outside: ReviewFinding[] = [];
  for (const f of findings) {
    const endAnchor = clampToAnchor(anchors, f.path, f.end_line ?? f.start_line);
    if (endAnchor === null) {
      outside.push(f);
      continue;
    }
    // Span both ends onto anchors, then order them: the two nearest anchors can
    // cross when the end snaps down and the start snaps up. Emitting max/min keeps
    // the range valid and preserves the reviewer's span instead of dropping it.
    let startAnchor = endAnchor;
    if (f.end_line != null && f.end_line !== f.start_line) {
      startAnchor = clampToAnchor(anchors, f.path, f.start_line) ?? endAnchor;
    }
    const line = Math.max(startAnchor, endAnchor);
    const start = Math.min(startAnchor, endAnchor);
    const comment: ReviewCommentInput = {
      path: f.path,
      line,
      side: 'RIGHT',
      body: formatFindingBody(f),
    };
    if (start < line) {
      comment.start_line = start;
      comment.start_side = 'RIGHT';
    }
    comments.push(comment);
  }
  return { comments, outside };
}

export function buildSummaryBody(markdown: string, outside: ReviewFinding[]): string {
  let body = markdown.trim();
  if (!body.includes(SUMMARY_ANCHOR)) body = `${SUMMARY_ANCHOR}\n${body}`;
  if (outside.length > 0) {
    const bullets = outside
      .map(
        (f) =>
          `- \`${f.path}:${f.end_line ?? f.start_line}\` [${f.severity}/${f.category}] ${f.title}`,
      )
      .join('\n');
    body += `\n\n### Findings outside the diff\n\n${bullets}`;
  }
  return body;
}

export function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

export function parseBatchSize(raw: string | undefined): number {
  const n = Number.parseInt(raw ?? '', 10);
  return Number.isInteger(n) && n > 0 ? n : DEFAULT_BATCH_SIZE;
}

/**
 * Parse the base-branch config. Null (absent file) yields empty lists. Malformed JSON
 * throws, so a broken base config fails the review visibly instead of silently reviewing
 * excluded files.
 */
export function parseReviewConfig(raw: string | null): ReviewConfig {
  if (raw == null) return { exclude: [], rules: [] };
  const obj = (JSON.parse(raw) ?? {}) as { exclude?: unknown; rules?: unknown };
  const exclude = Array.isArray(obj.exclude)
    ? obj.exclude.filter((p): p is string => typeof p === 'string')
    : [];
  const rules = Array.isArray(obj.rules)
    ? obj.rules
        .filter(
          (r): r is PathRule =>
            typeof r === 'object' &&
            r !== null &&
            typeof r.path === 'string' &&
            typeof r.rule === 'string',
        )
        .map((r) => ({ path: r.path, rule: r.rule }))
    : [];
  return { exclude, rules };
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

// ---------------------------------------------------------------------------
// GitHub client
// ---------------------------------------------------------------------------

class GitHubHttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

function toGitHubHttpError(err: unknown, method: string, path: string): GitHubHttpError {
  const status =
    typeof err === 'object' && err !== null && 'status' in err && typeof err.status === 'number'
      ? err.status
      : 0;
  const detail = err instanceof Error ? err.message : String(err);
  return new GitHubHttpError(status, `GitHub ${method} ${path} failed: ${detail}`);
}

const OctokitWithRetry = Octokit.plugin(retry);

/** Only these are safe to replay; a retried mutation can duplicate side effects. */
const IDEMPOTENT_METHODS = new Set(['GET', 'HEAD']);

export function createGitHubClient(token: string): GitHubClient {
  // Retry plugin defaults: 3 retries for 5xx/429/network, none for 4xx
  // (doNotRetry: 400,401,403,404,410,422,451). Do NOT set request.retries on the
  // client — that value lands on every error and defeats doNotRetry.
  const octokit = new OctokitWithRetry({ auth: token, request: { timeout: 30_000 } });
  const graphqlWithAuth = gql.defaults({
    headers: { authorization: `token ${token}` },
    request: { timeout: 30_000 },
  });

  const gh = async <T = unknown>(method: string, path: string, body?: unknown): Promise<T> => {
    const options: Record<string, unknown> = { ...((body ?? {}) as Record<string, unknown>) };
    // Never replay a mutating request: a lost-response retry would post a second
    // review/comment/reply. GET and HEAD are safe to retry.
    if (!IDEMPOTENT_METHODS.has(method.toUpperCase())) {
      options.request = { retries: 0 };
    }
    try {
      const res = await octokit.request(`${method} ${path}`, options);
      return res.data as T;
    } catch (err) {
      throw toGitHubHttpError(err, method, path);
    }
  };

  /** Paginate a list endpoint with per_page=100 until a short page is returned. */
  const list = async <T>(path: string): Promise<T[]> => {
    const sep = path.includes('?') ? '&' : '?';
    const out: T[] = [];
    const MAX_PAGES = 100; // 10,000 items at per_page=100
    for (let page = 1; page <= MAX_PAGES; page++) {
      const items = await gh<T[]>('GET', `${path}${sep}per_page=100&page=${page}`);
      out.push(...items);
      if (items.length < 100) return out;
    }
    console.warn(`pi-review: pagination hit ${MAX_PAGES} pages for ${path}; returning partial results`);
    return out;
  };

  const graphql = async <T>(query: string, variables: Record<string, unknown>): Promise<T> =>
    graphqlWithAuth<T>(query, variables);

  return { gh, list, graphql };
}

interface Runtime {
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

function loadRuntime(env: Env = process.env): Runtime {
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

interface PullPayload {
  title: string;
  base: { ref: string };
  head: { sha: string };
}

interface FilePayload {
  filename: string;
  status: string;
  patch?: string | null;
}

interface ReviewCommentPayload {
  id: number;
  node_id: string;
  in_reply_to_id?: number;
  body?: string | null;
  created_at: string;
  user?: { login?: string; type?: string } | null;
}

const repoPath = (rt: Runtime): string =>
  `/repos/${encodeURIComponent(rt.owner)}/${encodeURIComponent(rt.repo)}`;

async function fetchBaseConfig(api: GitHubClient, base: string, ref: string): Promise<string | null> {
  try {
    const file = await api.gh<{ type?: string; content?: string }>(
      'GET',
      `${base}/contents/${REVIEW_CONFIG_PATH}?ref=${encodeURIComponent(ref)}`,
    );
    return file.type === 'file' && file.content
      ? Buffer.from(file.content, 'base64').toString('utf8')
      : null;
  } catch (err) {
    if (err instanceof GitHubHttpError && err.status === 404) return null;
    throw err;
  }
}

/** One review run sees a fixed head, so the file list is safe to cache per PR+head. */
const changedFilesCache = new Map<string, FilePayload[]>();

async function getChangedFiles(
  api: GitHubClient,
  base: string,
  rt: Runtime,
  headSha: string,
): Promise<FilePayload[]> {
  const key = `${rt.owner}/${rt.repo}#${rt.prNumber}@${headSha}`;
  const cached = changedFilesCache.get(key);
  if (cached) return cached;
  const files = await api.list<FilePayload>(`${base}/pulls/${rt.prNumber}/files`);
  changedFilesCache.set(key, files);
  return files;
}

async function listReviewFiles(rt: Runtime): Promise<string> {
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const pr = await api.gh<PullPayload>('GET', `${base}/pulls/${rt.prNumber}`);
  const files = await getChangedFiles(api, base, rt, pr.head.sha);
  const config = parseReviewConfig(await fetchBaseConfig(api, base, pr.base.ref));

  const kept: Array<{ path: string; status: string; patch: string | null }> = [];
  const skippedExcluded: string[] = [];
  for (const f of files) {
    if (f.status === 'removed') continue;
    if (config.exclude.some((pattern) => matchesGlob(f.filename, pattern))) {
      skippedExcluded.push(f.filename);
      continue;
    }
    kept.push({ path: f.filename, status: f.status, patch: f.patch ?? null });
  }
  const rules = config.rules.filter((r) => kept.some((f) => matchesGlob(f.path, r.path)));
  return JSON.stringify(
    {
      prNumber: rt.prNumber,
      title: pr.title,
      baseRef: pr.base.ref,
      headSha: pr.head.sha,
      files: kept,
      skippedExcluded,
      rules,
    },
    null,
    2,
  );
}

async function getReviewThread(rt: Runtime, commentId: number): Promise<string> {
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

async function replyToReviewComment(rt: Runtime, commentId: number, body: string): Promise<string> {
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

async function resolveReviewThread(rt: Runtime, commentId: number): Promise<string> {
  const api = createGitHubClient(rt.token);
  const comment = await api.gh<{ node_id: string }>(
    'GET',
    `${repoPath(rt)}/pulls/comments/${commentId}`,
  );
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

async function upsertStickySummary(
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

async function postReview(rt: Runtime, params: PostReviewParams): Promise<string> {
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const { kept, dropped } = splitByConfidence(params.findings);
  const pr = await api.gh<PullPayload>('GET', `${base}/pulls/${rt.prNumber}`);
  const files = await getChangedFiles(api, base, rt, pr.head.sha);
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

  if (comments.length === 0) {
    await api.gh('POST', reviewPath, { commit_id: commitId, event: 'COMMENT', body: REVIEW_BODY_DONE });
  }

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
      // Fail-open: retry each comment of the failed batch as its own review.
      // Accepted risk: the batch POST is non-idempotent, so if GitHub committed
      // the batch but the response was lost, these per-comment posts duplicate
      // findings. Reconcile-by-fetch was rejected as more cost than the rare
      // failure warrants; duplicates are visible and can be resolved by hand.
      console.log(
        `pi-review post_review: batch ${i + 1} failed, retrying per comment: ${err instanceof Error ? err.message : String(err)}`,
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

// ---------------------------------------------------------------------------
// Pi extension wiring
// ---------------------------------------------------------------------------

interface ToolSpec {
  name: string;
  label: string;
  description: string;
  parameters: Record<string, unknown>;
  execute: (toolCallId: string, params: unknown) => Promise<ToolResult>;
}

interface ToolResult {
  content: Array<{ type: 'text'; text: string }>;
  details: Record<string, never>;
}

const textResult = (text: string): ToolResult => ({
  content: [{ type: 'text', text }],
  details: {},
});

function intParam(params: unknown, key: string): number {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 1) {
    throw new Error(`${key} must be a positive integer`);
  }
  return value;
}

function stringParam(params: unknown, key: string): string {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'string') throw new Error(`${key} must be a string`);
  return value;
}

/**
 * Validate the findings array before it reaches the comment builder, so a
 * malformed entry raises a clear tool error instead of an opaque runtime one.
 */
function parseFindings(value: unknown): ReviewFinding[] {
  if (!Array.isArray(value)) throw new Error('findings must be an array');
  return value.map((raw, i) => {
    const where = `findings[${i}]`;
    const f = (raw ?? {}) as Record<string, unknown>;
    if (typeof f.path !== 'string' || f.path === '') {
      throw new Error(`${where}.path must be a non-empty string`);
    }
    if (!Number.isInteger(f.start_line) || (f.start_line as number) < 1) {
      throw new Error(`${where}.start_line must be a positive integer`);
    }
    if (
      f.end_line != null &&
      (!Number.isInteger(f.end_line) || (f.end_line as number) < 1)
    ) {
      throw new Error(`${where}.end_line must be a positive integer when set`);
    }
    if (!SEVERITIES.includes(f.severity as Severity)) {
      throw new Error(`${where}.severity must be one of: ${SEVERITIES.join(', ')}`);
    }
    if (!CATEGORIES.includes(f.category as Category)) {
      throw new Error(`${where}.category must be one of: ${CATEGORIES.join(', ')}`);
    }
    if (typeof f.title !== 'string' || f.title === '') {
      throw new Error(`${where}.title must be a non-empty string`);
    }
    if (typeof f.body !== 'string' || f.body === '') {
      throw new Error(`${where}.body must be a non-empty string`);
    }
    if (typeof f.confidence !== 'number' || f.confidence < 0 || f.confidence > 1) {
      throw new Error(`${where}.confidence must be a number in [0, 1]`);
    }
    return {
      path: f.path,
      start_line: f.start_line as number,
      end_line: f.end_line as number | undefined,
      severity: f.severity as Severity,
      category: f.category as Category,
      title: f.title,
      body: f.body,
      confidence: f.confidence,
    };
  });
}

const commentIdSchema = {
  type: 'object',
  properties: { comment_id: { type: 'integer', minimum: 1 } },
  required: ['comment_id'],
};

export default function (pi: ExtensionAPI): void {
  // The reviewer model is not in Pi's built-in catalog, so register the OCR
  // endpoint as a provider. This must happen here (extensions load before the
  // action resolves the model), and the workflow must not pass base_url.
  const ocrModel = process.env.OCR_LLM_MODEL;
  // The SDK appends /chat/completions to baseUrl, so accept the full endpoint too.
  const ocrBaseUrl = process.env.OCR_LLM_URL?.replace(/\/chat\/completions\/?$/, '');
  if (!ocrModel || !ocrBaseUrl) {
    throw new Error('pi-review extension: OCR_LLM_MODEL and OCR_LLM_URL must be set');
  }
  pi.registerProvider('ocr-llm', {
    baseUrl: ocrBaseUrl,
    api: 'openai-completions',
    apiKey: '$OCR_LLM_AUTH_TOKEN',
    models: [
      {
        id: ocrModel,
        name: `OCR review model (${ocrModel})`,
        reasoning: false,
        input: ['text'],
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: 128000,
        maxTokens: 4096,
      },
    ],
  });

  const register = (tool: ToolSpec) =>
    pi.registerTool(tool as unknown as Parameters<ExtensionAPI['registerTool']>[0]);

  register({
    name: 'list_review_files',
    label: 'List review files',
    description:
      'List changed PR files to review (removed and excluded paths dropped) with base-branch path rules that apply to them.',
    parameters: { type: 'object', properties: {}, required: [] },
    execute: async () => textResult(await listReviewFiles(loadRuntime())),
  });

  register({
    name: 'get_review_thread',
    label: 'Get review thread',
    description:
      'Fetch the full inline review thread containing a PR review comment: root, replies (oldest first), and the comment node id.',
    parameters: commentIdSchema,
    execute: async (_id, params) =>
      textResult(await getReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
  });

  register({
    name: 'reply_to_review_comment',
    label: 'Reply to review comment',
    description: 'Reply in an inline PR review thread. The reply is tagged with the bot marker.',
    parameters: {
      type: 'object',
      properties: {
        comment_id: { type: 'integer' },
        body: { type: 'string' },
      },
      required: ['comment_id', 'body'],
    },
    execute: async (_id, params) =>
      textResult(
        await replyToReviewComment(
          loadRuntime(),
          intParam(params, 'comment_id'),
          stringParam(params, 'body'),
        ),
      ),
  });

  register({
    name: 'resolve_review_thread',
    label: 'Resolve review thread',
    description: 'Resolve the inline review thread containing the given PR review comment.',
    parameters: commentIdSchema,
    execute: async (_id, params) =>
      textResult(await resolveReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
  });

  register({
    name: 'post_review',
    label: 'Post review',
    description:
      'Post the review: drops low-confidence findings, anchors the rest to diff lines, upserts the sticky summary, and posts inline comments in batches.',
    parameters: {
      type: 'object',
      properties: {
        summary_markdown: { type: 'string' },
        findings: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              path: { type: 'string' },
              start_line: { type: 'integer', minimum: 1 },
              end_line: { type: 'integer', minimum: 1 },
              severity: { type: 'string', enum: [...SEVERITIES] },
              category: { type: 'string', enum: [...CATEGORIES] },
              title: { type: 'string' },
              body: { type: 'string' },
              confidence: { type: 'number', minimum: 0, maximum: 1 },
            },
            required: [
              'path',
              'start_line',
              'severity',
              'category',
              'title',
              'body',
              'confidence',
            ],
          },
        },
      },
      required: ['summary_markdown', 'findings'],
    },
    execute: async (_id, params) => {
      const p = (params ?? {}) as { findings?: unknown };
      return textResult(
        await postReview(loadRuntime(), {
          summary_markdown: stringParam(p, 'summary_markdown'),
          findings: parseFindings(p.findings),
        }),
      );
    },
  });
}
