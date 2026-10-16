import { Octokit } from '@octokit/rest';
import { retry } from '@octokit/plugin-retry';
import type { GitHubClient } from './types.ts';

export class GitHubHttpError extends Error {
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
    // Callers treat the result as authoritative (the sticky summary looks for an
    // existing anchor, a thread lookup looks for its root), so a truncated page
    // walk must not be mistaken for "not found".
    throw new Error(
      `pi-review: pagination exceeded ${MAX_PAGES} pages for ${path}; refusing to act on partial results`,
    );
  };

  // Reads may be replayed: THREAD_FOR_COMMENT_QUERY is read-only.
  const graphql = async <T>(query: string, variables: Record<string, unknown>): Promise<T> =>
    octokit.graphql<T>(query, variables);

  // Mutations are never replayed, matching the gh() rule above: a lost response
  // would otherwise re-issue a state change.
  const graphqlMutation = async <T>(
    query: string,
    variables: Record<string, unknown>,
  ): Promise<T> => octokit.graphql<T>(query, { ...variables, request: { retries: 0 } });

  return { gh, list, graphql, graphqlMutation };
}
