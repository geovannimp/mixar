import { existsSync, statSync } from 'node:fs';
import path from 'node:path';

import { execFileText, type ExecOptions, type ExecResult } from './exec.ts';

export type OcrAction = 'preview' | 'rule';

export interface OcrParams {
  action: OcrAction;
  from?: string;
  to?: string;
  commit?: string;
  paths?: string[];
  repo?: string;
  rule?: string;
}

export interface OcrPreviewFile {
  path: string;
  status: string;
  insertions: number;
  deletions: number;
}

export interface OcrPreviewExcluded extends OcrPreviewFile {
  reason: string;
}

export interface OcrPreview {
  action: 'preview';
  mode: string;
  reviewable: number;
  total: number;
  /** The rule file the review ran under. */
  rule: string;
  from?: string;
  to?: string;
  commit?: string;
  merge_base?: string;
  total_insertions?: number;
  total_deletions?: number;
  files: OcrPreviewFile[];
  excluded: OcrPreviewExcluded[];
  raw: string;
}

export interface OcrRuleGroup {
  name: string;
  appliesTo: string[];
  content: string;
}

export interface OcrRules {
  action: 'rule';
  /** The rule file the review ran under. */
  rule: string;
  groups: OcrRuleGroup[];
  raw: string;
}

export type OcrResult = OcrPreview | OcrRules;

export interface OcrDeps {
  workspace: string;
  run: (
    cmd: string,
    args: string[],
    options: ExecOptions,
  ) => Promise<ExecResult>;
  exists: (candidate: string) => boolean;
  stat: (candidate: string) => { isDirectory(): boolean; isFile(): boolean };
  bin?: string;
  env?: NodeJS.ProcessEnv;
}

/** Refs handed to execFile never reach a shell, but a leading '-' would be read as a flag. */
const REF = /^[A-Za-z0-9_+][A-Za-z0-9._/@{}^~+-]*$/;
const COMMIT = /^[0-9a-fA-F]{4,64}$/;

export function defaultOcrDeps(env: NodeJS.ProcessEnv = process.env): OcrDeps {
  return {
    workspace: path.resolve(env.GITHUB_WORKSPACE || process.cwd()),
    run: execFileText,
    exists: existsSync,
    stat: statSync,
    bin: env.OCR_BIN || 'ocr',
    env,
  };
}

function validRef(name: string, value: unknown, required: boolean): string | undefined {
  if (value === undefined || value === null || value === '') {
    if (required) throw new Error(`${name} is required`);
    return undefined;
  }
  if (typeof value !== 'string' || !REF.test(value) || value.includes('..')) {
    throw new Error(`${name} must be a git ref without whitespace, a leading '-', or '..'`);
  }
  return value;
}

function validCommit(value: unknown): string {
  if (typeof value !== 'string' || !COMMIT.test(value)) {
    throw new Error('commit must be a 4-64 character hex SHA');
  }
  return value;
}

function validPaths(value: unknown): string[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error('paths must be a non-empty array of repo-relative paths');
  }
  let total = 0;
  const paths: string[] = [];
  for (const entry of value) {
    if (typeof entry !== 'string' || entry.length === 0 || entry.length > 4096) {
      throw new Error('each path must be a non-empty string of at most 4096 characters');
    }
    if (entry.startsWith('-') || /[\0\r\n]/.test(entry)) {
      throw new Error(`invalid path: ${JSON.stringify(entry)}`);
    }
    if (path.isAbsolute(entry) || entry.split(/[\\/]/).includes('..')) {
      throw new Error(`paths must be repo-relative and may not contain '..': ${entry}`);
    }
    total += entry.length;
    paths.push(entry);
  }
  if (total > 100_000) throw new Error('paths are too long');
  return paths;
}

function inside(root: string, candidate: string): boolean {
  const rel = path.relative(root, candidate);
  return rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
}

function resolveInside(
  workspace: string,
  candidate: unknown,
  label: string,
  deps: OcrDeps,
): string {
  if (typeof candidate !== 'string' || candidate.length === 0 || candidate.includes('\0')) {
    throw new Error(`${label} must be a non-empty path`);
  }
  const resolved = path.resolve(workspace, candidate);
  if (!inside(workspace, resolved)) throw new Error(`${label} must stay inside the workspace`);
  if (!deps.exists(resolved)) throw new Error(`${label} does not exist: ${resolved}`);
  return resolved;
}

function resolveRepo(workspace: string, params: OcrParams, deps: OcrDeps): string {
  if (params.repo === undefined) return workspace;
  const repo = resolveInside(workspace, params.repo, 'repo', deps);
  if (!deps.stat(repo).isDirectory()) throw new Error(`repo is not a directory: ${repo}`);
  return repo;
}

/**
 * The rule file is pinned by the workflow through OCR_RULE_FILE, which holds the
 * base branch's copy extracted before the review. A model-supplied `rule` never
 * wins over the pin, so a PR cannot judge itself with rules it authored.
 */
function resolveRule(workspace: string, params: OcrParams, deps: OcrDeps): string {
  const pinned = deps.env?.OCR_RULE_FILE?.trim();
  if (pinned) {
    const resolved = path.resolve(pinned);
    if (!inside(workspace, resolved)) {
      throw new Error(`OCR_RULE_FILE must be inside the workspace: ${resolved}`);
    }
    if (!deps.exists(resolved)) throw new Error(`OCR_RULE_FILE does not exist: ${resolved}`);
    if (!deps.stat(resolved).isFile()) throw new Error(`OCR_RULE_FILE is not a file: ${resolved}`);
    return resolved;
  }
  if (params.rule !== undefined) {
    const rule = resolveInside(workspace, params.rule, 'rule', deps);
    if (!deps.stat(rule).isFile()) throw new Error(`rule is not a file: ${rule}`);
    return rule;
  }
  const conventional = path.join(workspace, '.opencodereview', 'rule.json');
  if (!deps.exists(conventional)) {
    throw new Error(
      'no trusted review rules: restore .opencodereview/rule.json or pass rule',
    );
  }
  if (!deps.stat(conventional).isFile()) {
    throw new Error(`rule is not a file: ${conventional}`);
  }
  return conventional;
}

export function buildOcrArgs(
  params: OcrParams,
  deps: OcrDeps,
): { args: string[]; cwd: string; rule: string } {
  if (params.action !== 'preview' && params.action !== 'rule') {
    throw new Error('action must be "preview" or "rule"');
  }
  const workspace = path.resolve(deps.workspace);
  const repo = resolveRepo(workspace, params, deps);
  const rule = resolveRule(workspace, params, deps);
  const args = ['delegate', params.action];

  if (params.action === 'preview') {
    const hasRange = params.from !== undefined || params.to !== undefined;
    if (hasRange && params.commit !== undefined) {
      throw new Error('pass either from/to or commit, not both');
    }
    if (hasRange) {
      const from = validRef('from', params.from, true);
      const to = validRef('to', params.to, true);
      args.push('--from', from!, '--to', to!);
    } else if (params.commit !== undefined) {
      args.push('--commit', validCommit(params.commit));
    }
    args.push('--repo', repo);
    args.push('--rule', rule);
    return { args, cwd: repo, rule };
  }

  const paths = validPaths(params.paths);
  args.push('--repo', repo);
  args.push('--rule', rule);
  args.push(...paths);
  return { args, cwd: repo, rule };
}

export function parsePreview(raw: string, rule = ''): OcrPreview {
  const header = raw.match(/^# Files \((\d+) reviewable \/ (\d+) total\)$/m);
  if (!header) {
    throw new Error('unexpected `ocr delegate preview` output: missing "# Files (...)" header');
  }
  const meta = (key: string): string | undefined =>
    raw.match(new RegExp(`^- ${key}: (.*)$`, 'm'))?.[1]?.trim();
  const number = (key: string): number | undefined => {
    const value = meta(key);
    return value && /^\d+$/.test(value) ? Number(value) : undefined;
  };

  const files: OcrPreviewFile[] = [];
  const excluded: OcrPreviewExcluded[] = [];
  for (const line of raw.split('\n')) {
    const skipped = line.match(
      /^~~- `([^`]+)` \[([^\]]+)\] \+(\d+)\/-(\d+) \(excluded: ([^)]+)\)~~\s*$/,
    );
    if (skipped) {
      excluded.push({
        path: skipped[1]!,
        status: skipped[2]!,
        insertions: Number(skipped[3]!),
        deletions: Number(skipped[4]!),
        reason: skipped[5]!,
      });
      continue;
    }
    const file = line.match(/^\s*- `([^`]+)` \[([^\]]+)\] \+(\d+)\/-(\d+)\s*$/);
    if (file) {
      files.push({
        path: file[1]!,
        status: file[2]!,
        insertions: Number(file[3]!),
        deletions: Number(file[4]!),
      });
    }
  }

  const reviewable = Number(header[1]);
  const total = Number(header[2]);
  if (files.length !== reviewable) {
    throw new Error(
      `unexpected \`ocr delegate preview\` output: header lists ${reviewable} reviewable files but parsed ${files.length}`,
    );
  }
  // Workspace mode also reviews untracked files, which the totals do not cover.
  if (meta('mode') !== 'workspace' && files.length + excluded.length !== total) {
    throw new Error(
      `unexpected \`ocr delegate preview\` output: header lists ${total} files but parsed ${files.length} reviewable and ${excluded.length} excluded`,
    );
  }

  const preview: OcrPreview = {
    action: 'preview',
    mode: meta('mode') ?? 'unknown',
    reviewable,
    total,
    rule,
    files,
    excluded,
    raw,
  };
  const from = meta('from');
  const to = meta('to');
  const commit = meta('commit');
  const mergeBase = meta('merge_base');
  const insertions = number('total_insertions');
  const deletions = number('total_deletions');
  if (from) preview.from = from;
  if (to) preview.to = to;
  if (commit) preview.commit = commit;
  if (mergeBase) preview.merge_base = mergeBase;
  if (insertions !== undefined) preview.total_insertions = insertions;
  if (deletions !== undefined) preview.total_deletions = deletions;
  return preview;
}

export function parseRules(raw: string, rule = ''): OcrRules {
  const starts = Array.from(raw.matchAll(/^### Rule Group \d+: (.*)$/gm));
  const groups: OcrRuleGroup[] = [];
  for (let index = 0; index < starts.length; index += 1) {
    const start = (starts[index]!.index ?? 0) + starts[index]![0].length;
    const end = index + 1 < starts.length ? (starts[index + 1]!.index ?? raw.length) : raw.length;
    const block = raw.slice(start, end);
    const applies = block.match(/^Applies to:\n((?:^- .+\n?)+)/m);
    const appliesTo = applies
      ? applies[1]!
          .split('\n')
          .map((line) => line.trim())
          .filter((line) => line.startsWith('- '))
          .map((line) => line.slice(2).trim())
      : [];
    const content = (block.split(/^#### Content\s*$/m)[1] ?? '')
      .replace(/\n---\s*$/, '')
      .trim();
    if (!content) {
      throw new Error(
        `unexpected \`ocr delegate rule\` output: empty content for rule group ${starts[index]![1]}`,
      );
    }
    groups.push({ name: starts[index]![1]!.trim(), appliesTo, content });
  }
  if (groups.length === 0) {
    throw new Error('unexpected `ocr delegate rule` output: no rule groups');
  }
  return { action: 'rule', rule, groups, raw };
}

export async function executeOcr(
  params: OcrParams,
  deps: OcrDeps = defaultOcrDeps(),
): Promise<OcrResult> {
  const { args, cwd, rule } = buildOcrArgs(params, deps);
  const sourceEnv = deps.env ?? process.env;
  const env: NodeJS.ProcessEnv = {
    PATH: sourceEnv.PATH ?? '',
    HOME: sourceEnv.HOME ?? '',
    LANG: sourceEnv.LANG ?? 'C.UTF-8',
    LC_ALL: sourceEnv.LC_ALL ?? '',
    TMPDIR: sourceEnv.TMPDIR ?? '',
    OCR_NO_UPDATE: '1',
  };

  let result: ExecResult;
  try {
    result = await deps.run(deps.bin ?? 'ocr', args, {
      cwd,
      env,
      timeout: 120_000,
      maxBuffer: 4 * 1024 * 1024,
      shell: false,
    });
  } catch (error) {
    const failure = error as Error & { code?: string; stderr?: string };
    if (failure.code === 'ENOENT') {
      throw new Error(
        'ocr not found: install @alibaba-group/open-code-review@1.13.2 (or set OCR_BIN)',
      );
    }
    const stderr = (failure.stderr ?? '').trim().slice(0, 2000);
    throw new Error(
      `ocr ${params.action} failed${failure.code ? ` (${failure.code})` : ''}: ${
        stderr || failure.message || 'unknown error'
      }`,
    );
  }

  return params.action === 'preview'
    ? parsePreview(result.stdout, rule)
    : parseRules(result.stdout, rule);
}
