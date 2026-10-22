import path from 'node:path';

import { execFileText, type ExecOptions, type ExecResult } from './exec.ts';

/**
 * The CodeGraph CLI verbs the review uses. `explore` is the primary one: it
 * returns the verbatim source of the relevant symbols plus their call paths
 * and blast radius, which is what the reviewer needs to judge a diff.
 */
export type CodegraphAction = 'explore' | 'node' | 'callers' | 'callees' | 'impact';

export interface CodegraphParams {
  action: CodegraphAction;
  /**
   * `explore`: symbol names, file names, or a natural-language question.
   * The other actions: the symbol to look up.
   */
  query: string;
  /** Narrow a symbol lookup to the definitions in one file. */
  file?: string;
  /** Maximum results per definition, for the callers/callees verbs. */
  limit?: number;
  /** Traversal depth, for the impact verb. */
  depth?: number;
  /** Maximum files to include source from, for the explore verb. */
  maxFiles?: number;
}

export interface CodegraphDeps {
  /** The project to query: the root of the tree CodeGraph indexed. */
  project: string;
  run: (
    cmd: string,
    args: string[],
    options: ExecOptions,
  ) => Promise<ExecResult>;
  bin?: string;
  env?: NodeJS.ProcessEnv;
}

export interface CodegraphResult {
  action: CodegraphAction;
  output: string;
}

const ACTIONS: readonly CodegraphAction[] = [
  'explore',
  'node',
  'callers',
  'callees',
  'impact',
];

/** Which optional arguments each CLI verb actually accepts. */
const OPTIONS: Record<
  CodegraphAction,
  Record<'file' | 'limit' | 'depth' | 'maxFiles', boolean>
> = {
  explore: { file: false, limit: false, depth: false, maxFiles: true },
  node: { file: true, limit: false, depth: false, maxFiles: false },
  callers: { file: true, limit: true, depth: false, maxFiles: false },
  callees: { file: true, limit: true, depth: false, maxFiles: false },
  impact: { file: true, limit: false, depth: true, maxFiles: false },
};

/** A query is free text, so only the flags that would change its meaning are rejected. */
const CONTROL = /[\0\r\n]/;

export function defaultCodegraphDeps(env: NodeJS.ProcessEnv = process.env): CodegraphDeps {
  return {
    project: path.resolve(env.CODEGRAPH_PROJECT || process.cwd()),
    run: execFileText,
    bin: env.CODEGRAPH_BIN || 'codegraph',
    env,
  };
}

function validQuery(value: unknown): string {
  if (typeof value !== 'string') throw new Error('query must be a string');
  const query = value.trim();
  if (query.length === 0) throw new Error('query must not be empty');
  if (query.length > 4096) throw new Error('query must be at most 4096 characters');
  if (CONTROL.test(query)) throw new Error('query must not contain control characters');
  return query;
}

/**
 * A file narrows a symbol lookup, so it must stay inside the indexed project:
 * an absolute path or a `..` segment would point CodeGraph at a tree the
 * review has no business reading.
 */
function validFile(project: string, value: unknown): string | undefined {
  if (value === undefined || value === null || value === '') return undefined;
  if (typeof value !== 'string') throw new Error('file must be a string');
  const file = value.trim();
  if (file.length === 0) return undefined;
  if (CONTROL.test(file)) throw new Error('file must not contain control characters');
  if (path.isAbsolute(file)) throw new Error('file must be relative to the project');
  const resolved = path.resolve(project, file);
  const relative = path.relative(project, resolved);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error('file must stay inside the project');
  }
  return relative;
}

function validCount(
  name: string,
  value: unknown,
  min: number,
  max: number,
): number | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== 'number' || !Number.isInteger(value)) {
    throw new Error(`${name} must be an integer`);
  }
  if (value < min || value > max) {
    throw new Error(`${name} must be between ${min} and ${max}`);
  }
  return value;
}

/**
 * Builds the CodeGraph CLI invocation. Every model-supplied value is passed
 * after a `--` separator so it can never be read as a flag, and the project
 * path is fixed by the caller rather than the model.
 */
export function buildCodegraphArgs(
  params: CodegraphParams,
  deps: CodegraphDeps,
): { args: string[]; cwd: string } {
  if (!ACTIONS.includes(params.action)) {
    throw new Error(`action must be one of ${ACTIONS.join(', ')}`);
  }
  const query = validQuery(params.query);
  const project = path.resolve(deps.project);
  const args: string[] = [params.action, '-p', project];

  // Each verb takes a different set of options, so an option that the verb
  // does not have is rejected rather than silently dropped: a silently
  // dropped `--file` would widen the lookup the caller asked to narrow.
  const options = OPTIONS[params.action];
  for (const name of ['file', 'limit', 'depth', 'maxFiles'] as const) {
    if (params[name] !== undefined && !options[name]) {
      throw new Error(`${name} does not apply to ${params.action}`);
    }
  }

  const file = validFile(project, params.file);
  if (file !== undefined) args.push('--file', file);
  if (params.action === 'explore') {
    const maxFiles = validCount('maxFiles', params.maxFiles, 1, 100);
    if (maxFiles !== undefined) args.push('--max-files', String(maxFiles));
  }
  if (params.action === 'callers' || params.action === 'callees') {
    const limit = validCount('limit', params.limit, 1, 100);
    if (limit !== undefined) args.push('--limit', String(limit));
  }
  if (params.action === 'impact') {
    const depth = validCount('depth', params.depth, 1, 10);
    if (depth !== undefined) args.push('--depth', String(depth));
  }
  // The separator keeps the query out of the flag namespace.
  args.push('--', query);
  return { args, cwd: project };
}

export async function executeCodegraph(
  params: CodegraphParams,
  deps: CodegraphDeps = defaultCodegraphDeps(),
): Promise<CodegraphResult> {
  const { args, cwd } = buildCodegraphArgs(params, deps);
  const sourceEnv = deps.env ?? process.env;
  const env: NodeJS.ProcessEnv = {
    PATH: sourceEnv.PATH ?? '',
    HOME: sourceEnv.HOME ?? '',
    LANG: sourceEnv.LANG ?? 'C.UTF-8',
    LC_ALL: sourceEnv.LC_ALL ?? '',
    TMPDIR: sourceEnv.TMPDIR ?? '',
    // The npm shim downloads a platform bundle when the optional dependency
    // is missing; forbid that so the review never fetches code at run time.
    CODEGRAPH_NO_DOWNLOAD: '1',
  };

  let result: ExecResult;
  try {
    result = await deps.run(deps.bin ?? 'codegraph', args, {
      cwd,
      env,
      timeout: 120_000,
      maxBuffer: 8 * 1024 * 1024,
      shell: false,
    });
  } catch (error) {
    const failure = error as Error & { code?: string; stderr?: string };
    if (failure.code === 'ENOENT') {
      throw new Error(
        'codegraph not found: install @colbymchenry/codegraph@1.6.2 (or set CODEGRAPH_BIN)',
      );
    }
    const stderr = (failure.stderr ?? '').trim().slice(0, 2000);
    throw new Error(
      `codegraph ${params.action} failed${failure.code ? ` (${failure.code})` : ''}: ${
        stderr || failure.message || 'unknown error'
      }`,
    );
  }

  return { action: params.action, output: result.stdout };
}
