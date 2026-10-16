import { REVIEW_CONFIG_PATH } from './constants.ts';
import { parseReviewConfig } from './config.ts';
import { matchesGlob } from './diff.ts';
import { GitHubHttpError, createGitHubClient } from './github.ts';
import { repoPath, type Runtime } from './runtime.ts';
import type { FilePayload, GitHubClient, PullPayload, ReviewConfig } from './types.ts';

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

/**
 * Drop files the reviewer may not comment on: deleted files have no RIGHT-side
 * lines, and config exclusions (lockfiles, generated code, binaries) must hold for
 * inline anchoring too, not just for the file list handed to the model.
 */
export function splitReviewable(
  files: FilePayload[],
  config: ReviewConfig,
): { kept: FilePayload[]; skippedExcluded: string[] } {
  const kept: FilePayload[] = [];
  const skippedExcluded: string[] = [];
  for (const f of files) {
    if (f.status === 'removed') continue;
    if (config.exclude.some((pattern) => matchesGlob(f.filename, pattern))) {
      skippedExcluded.push(f.filename);
      continue;
    }
    kept.push(f);
  }
  return { kept, skippedExcluded };
}

/** Changed files that inline comments may be anchored to. */
export async function reviewableChangedFiles(
  api: GitHubClient,
  base: string,
  rt: Runtime,
  headSha: string,
  baseRef: string,
): Promise<FilePayload[]> {
  const files = await getChangedFiles(api, base, rt, headSha);
  const config = parseReviewConfig(await fetchBaseConfig(api, base, baseRef));
  return splitReviewable(files, config).kept;
}

export async function listReviewFiles(rt: Runtime): Promise<string> {
  const api = createGitHubClient(rt.token);
  const base = repoPath(rt);
  const pr = await api.gh<PullPayload>('GET', `${base}/pulls/${rt.prNumber}`);
  const files = await getChangedFiles(api, base, rt, pr.head.sha);
  const config = parseReviewConfig(await fetchBaseConfig(api, base, pr.base.ref));

  const { kept, skippedExcluded } = splitReviewable(files, config);
  const rules = config.rules.filter((r) =>
    kept.some((f) => matchesGlob(f.filename, r.path)),
  );
  return JSON.stringify(
    {
      prNumber: rt.prNumber,
      title: pr.title,
      baseRef: pr.base.ref,
      headSha: pr.head.sha,
      files: kept.map((f) => ({ path: f.filename, status: f.status, patch: f.patch ?? null })),
      skippedExcluded,
      rules,
    },
    null,
    2,
  );
}
