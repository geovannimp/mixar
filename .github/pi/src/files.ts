import { REVIEW_CONFIG_PATH } from './constants.ts';
import { parseReviewConfig } from './config.ts';
import { matchesGlob } from './diff.ts';
import { GitHubHttpError, createGitHubClient } from './github.ts';
import { repoPath, type Runtime } from './runtime.ts';
import type { FilePayload, GitHubClient, PullPayload } from './types.ts';

export async function fetchBaseConfig(
  api: GitHubClient,
  base: string,
  ref: string,
): Promise<string | null> {
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

export async function getChangedFiles(
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

export async function listReviewFiles(rt: Runtime): Promise<string> {
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
