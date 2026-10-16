import { REVIEW_CONFIG_PATH } from './constants.ts';
import { parseReviewConfig } from './config.ts';
import { matchesGlob } from './diff.ts';
import { createOctokit, httpStatus, type GitHubOctokit } from './github.ts';
import type { Runtime } from './runtime.ts';
import type { FilePayload, ReviewConfig } from './types.ts';

export async function fetchBaseConfig(
  octokit: GitHubOctokit,
  rt: Runtime,
  ref: string,
): Promise<string | null> {
  try {
    const { data } = await octokit.rest.repos.getContent({
      owner: rt.owner,
      repo: rt.repo,
      path: REVIEW_CONFIG_PATH,
      ref,
    });
    // This endpoint also returns directory listings and symlinks; only the file
    // variant carries base64 content.
    if (Array.isArray(data) || data.type !== 'file' || !('content' in data) || !data.content) {
      return null;
    }
    return Buffer.from(data.content, 'base64').toString('utf8');
  } catch (err) {
    if (httpStatus(err) === 404) return null;
    throw err;
  }
}

/** One review run sees a fixed head, so the file list is safe to cache per PR+head. */
const changedFilesCache = new Map<string, FilePayload[]>();

export async function getChangedFiles(
  octokit: GitHubOctokit,
  rt: Runtime,
  headSha: string,
): Promise<FilePayload[]> {
  const key = `${rt.owner}/${rt.repo}#${rt.prNumber}@${headSha}`;
  const cached = changedFilesCache.get(key);
  if (cached) return cached;
  // paginate follows every page, so the result is never a silent partial.
  const files = await octokit.paginate(octokit.rest.pulls.listFiles, {
    owner: rt.owner,
    repo: rt.repo,
    pull_number: rt.prNumber,
    per_page: 100,
  });
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
  octokit: GitHubOctokit,
  rt: Runtime,
  headSha: string,
  baseRef: string,
): Promise<FilePayload[]> {
  const files = await getChangedFiles(octokit, rt, headSha);
  const config = parseReviewConfig(await fetchBaseConfig(octokit, rt, baseRef));
  return splitReviewable(files, config).kept;
}

export async function listReviewFiles(rt: Runtime): Promise<string> {
  const octokit = createOctokit(rt.token);
  const { data: pr } = await octokit.rest.pulls.get({
    owner: rt.owner,
    repo: rt.repo,
    pull_number: rt.prNumber,
  });
  const files = await getChangedFiles(octokit, rt, pr.head.sha);
  const config = parseReviewConfig(await fetchBaseConfig(octokit, rt, pr.base.ref));

  const { kept, skippedExcluded } = splitReviewable(files, config);
  const rules = config.rules.filter((r) => kept.some((f) => matchesGlob(f.filename, r.path)));
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
