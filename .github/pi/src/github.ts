import { Octokit } from '@octokit/rest';
import { retry } from '@octokit/plugin-retry';

const OctokitWithRetry = Octokit.plugin(retry);

export type GitHubOctokit = InstanceType<typeof OctokitWithRetry>;

/** HTTP status of a failed request, or 0 for a network/unknown error. */
export function httpStatus(err: unknown): number {
  return typeof err === 'object' && err !== null && 'status' in err && typeof err.status === 'number'
    ? err.status
    : 0;
}

/** Methods safe to replay; every other method is pinned to zero retries. */
const IDEMPOTENT_METHODS = new Set(['GET', 'HEAD']);

/**
 * Octokit for this action: 30s timeout, with mutations never replayed.
 *
 * Retry plugin defaults: 3 retries for 5xx/429/network, none for 4xx
 * (doNotRetry: 400,401,403,404,410,422,451). Do NOT set request.retries on the
 * client itself — that value lands on every error and defeats doNotRetry.
 *
 * A replayed POST would post a second review or comment, or resolve a thread
 * twice, and a lost response is indistinguishable from a rejection. The retry
 * plugin reads options.request.retries, so pin it to 0 for non-idempotent
 * methods here, before the plugin sees the request. GitHub's GraphQL API is
 * always POST, so GraphQL calls do not retry either: that is the policy, not an
 * oversight.
 */
export function createOctokit(token: string): GitHubOctokit {
  const octokit = new OctokitWithRetry({ auth: token, request: { timeout: 30_000 } });
  octokit.hook.wrap('request', (request, options) => {
    if (!IDEMPOTENT_METHODS.has(options.method.toUpperCase())) {
      options.request = { ...(options.request ?? {}), retries: 0 };
    }
    return request(options);
  });
  return octokit;
}
