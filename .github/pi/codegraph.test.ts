import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  buildCodegraphArgs,
  executeCodegraph,
  type CodegraphDeps,
  type CodegraphParams,
} from './src/codegraph.ts';
import type { ExecResult } from './src/exec.ts';

function fixture(): { project: string } {
  return { project: mkdtempSync(join(tmpdir(), 'codegraph-tool-')) };
}

function fakeDeps(
  project: string,
  output = '**Exploration: render**\n',
): {
  deps: CodegraphDeps;
  calls: Array<{ cmd: string; args: string[]; env: NodeJS.ProcessEnv }>;
} {
  const calls: Array<{ cmd: string; args: string[]; env: NodeJS.ProcessEnv }> = [];
  const deps: CodegraphDeps = {
    project,
    bin: 'codegraph',
    env: { PATH: '/usr/bin', HOME: '/home/test', GITHUB_TOKEN: 'secret' },
    run: async (cmd, args, options): Promise<ExecResult> => {
      calls.push({ cmd, args, env: options.env });
      return { stdout: output, stderr: '' };
    },
  };
  return { deps, calls };
}

test('explore: builds the CLI args for the indexed project', async () => {
  const { project } = fixture();
  const { deps, calls } = fakeDeps(project);

  const result = await executeCodegraph(
    { action: 'explore', query: 'engine deck render' },
    deps,
  );

  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.cmd, 'codegraph');
  assert.deepEqual(calls[0]!.args, [
    'explore',
    '-p',
    project,
    '--',
    'engine deck render',
  ]);
  assert.equal(result.action, 'explore');
  assert.equal(result.output, '**Exploration: render**\n');
});

test('the query is passed after a separator so it cannot become a flag', () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);

  const { args } = buildCodegraphArgs(
    { action: 'explore', query: '--help' },
    deps,
  );

  assert.deepEqual(args, ['explore', '-p', project, '--', '--help']);
});

test('the optional arguments reach the right verb', () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);

  assert.deepEqual(
    buildCodegraphArgs({ action: 'explore', query: 'x', maxFiles: 3 }, deps).args,
    ['explore', '-p', project, '--max-files', '3', '--', 'x'],
  );
  assert.deepEqual(
    buildCodegraphArgs({ action: 'callers', query: 'render', limit: 5 }, deps).args,
    ['callers', '-p', project, '--limit', '5', '--', 'render'],
  );
  assert.deepEqual(
    buildCodegraphArgs({ action: 'callees', query: 'render' }, deps).args,
    ['callees', '-p', project, '--', 'render'],
  );
  assert.deepEqual(
    buildCodegraphArgs({ action: 'impact', query: 'render', depth: 2 }, deps).args,
    ['impact', '-p', project, '--depth', '2', '--', 'render'],
  );
  assert.deepEqual(
    buildCodegraphArgs({ action: 'node', query: 'render' }, deps).args,
    ['node', '-p', project, '--', 'render'],
  );
});

test('a file narrows the lookup and stays inside the project', () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);

  assert.deepEqual(
    buildCodegraphArgs(
      { action: 'callers', query: 'render', file: 'crates/engine-core/src/callback.rs' },
      deps,
    ).args,
    [
      'callers',
      '-p',
      project,
      '--file',
      'crates/engine-core/src/callback.rs',
      '--',
      'render',
    ],
  );
});

test('rejects unsafe or incomplete arguments', () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);
  const bad: CodegraphParams[] = [
    { action: 'explore', query: '' },
    { action: 'explore', query: 'a\nb' },
    { action: 'explore', query: 'x'.repeat(4097) },
    { action: 'explore', query: 'x', file: '../outside.rs' },
    { action: 'explore', query: 'x', file: '/etc/passwd' },
    { action: 'explore', query: 'x', maxFiles: 0 },
    { action: 'impact', query: 'x', depth: 99 },
    { action: 'callers', query: 'x', limit: 1.5 },
  ];
  for (const params of bad) {
    assert.throws(
      () => buildCodegraphArgs(params, deps),
      Error,
      JSON.stringify(params),
    );
  }
  assert.throws(
    () =>
      buildCodegraphArgs(
        { action: 'search' as CodegraphParams['action'], query: 'x' },
        deps,
      ),
    /action must be one of/,
  );
});

test('an option the verb does not take is rejected, not dropped', () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);

  // `explore` has no --file, so accepting one would widen the lookup the
  // caller asked to narrow.
  assert.throws(
    () =>
      buildCodegraphArgs(
        { action: 'explore', query: 'x', file: 'crates/engine-core/src/callback.rs' },
        deps,
      ),
    /file does not apply to explore/,
  );
  assert.throws(
    () => buildCodegraphArgs({ action: 'callers', query: 'x', maxFiles: 3 }, deps),
    /maxFiles/,
  );
});

test('the child process does not inherit the review tokens', async () => {
  const { project } = fixture();
  const { deps, calls } = fakeDeps(project);

  await executeCodegraph({ action: 'explore', query: 'render' }, deps);

  assert.equal(calls[0]!.env.GITHUB_TOKEN, undefined);
  assert.equal(calls[0]!.env.CODEGRAPH_NO_DOWNLOAD, '1');
});

test('execute: a missing binary and a failed run surface actionable errors', async () => {
  const { project } = fixture();
  const { deps } = fakeDeps(project);
  deps.run = async () => {
    throw Object.assign(new Error('spawn codegraph ENOENT'), { code: 'ENOENT' });
  };
  await assert.rejects(
    executeCodegraph({ action: 'explore', query: 'render' }, deps),
    /codegraph not found: install/,
  );

  deps.run = async () => {
    throw Object.assign(new Error('exit 1'), { code: 1, stderr: 'no index\n' });
  };
  await assert.rejects(
    executeCodegraph({ action: 'explore', query: 'render' }, deps),
    /codegraph explore failed \(1\): no index/,
  );
});
