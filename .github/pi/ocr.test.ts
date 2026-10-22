import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { execFileText, type ExecResult } from './src/exec.ts';
import {
  buildOcrArgs,
  executeOcr,
  parsePreview,
  parseRules,
  type OcrDeps,
  type OcrParams,
} from './src/ocr.ts';
import { registerTools } from './src/tools.ts';

const PREVIEW = `# Files (2 reviewable / 4 total)

- mode: range
- from: main
- to: HEAD
- merge_base: abc123
- total_insertions: 10
- total_deletions: 4

  - \`src/a.rs\` [modified] +6/-2
  - \`src/b.rs\` [added] +4/-0
~~- \`src/deleted.rs\` [deleted] +0/-9 (excluded: deleted)~~
~~- \`src/gen.rs\` [modified] +1/-1 (excluded: user_exclude)~~
`;

const RULES = `### Rule Group 1: system / default

Applies to:
- src/a.dart

#### Content

Be correct.

---

### Rule Group 2: system / **/*.rs

Applies to:
- src/b.rs

#### Content

No unwrap.
`;

function fixture(withPrHead = true, withRule = true) {
  const workspace = mkdtempSync(join(tmpdir(), 'ocr-tool-'));
  const prHead = join(workspace, 'pr-head');
  if (withPrHead) mkdirSync(prHead);
  const rule = join(workspace, '.opencodereview', 'rule.json');
  if (withRule) {
    mkdirSync(join(workspace, '.opencodereview'));
    writeFileSync(rule, '{}');
  }
  return { workspace, prHead, rule };
}

function fakeDeps(
  workspace: string,
  output = PREVIEW,
): { deps: OcrDeps; calls: Array<{ cmd: string; args: string[]; env: NodeJS.ProcessEnv }> } {
  const calls: Array<{ cmd: string; args: string[]; env: NodeJS.ProcessEnv }> = [];
  const deps: OcrDeps = {
    workspace,
    bin: 'ocr',
    env: {
      PATH: '/usr/bin',
      HOME: '/home/test',
      GITHUB_TOKEN: 'secret',
      OCR_LLM_TOKEN: 'secret',
    },
    exists: existsSync,
    stat: statSync,
    run: async (cmd, args, options): Promise<ExecResult> => {
      calls.push({ cmd, args, env: options.env });
      return { stdout: output, stderr: '' };
    },
  };
  return { deps, calls };
}

test('preview: builds range args against the workspace and the trusted rule file', async () => {
  const { workspace, rule } = fixture();
  const { deps, calls } = fakeDeps(workspace);

  const result = await executeOcr({ action: 'preview', from: 'main', to: 'HEAD' }, deps);

  assert.equal(calls.length, 1);
  assert.equal(calls[0]!.cmd, 'ocr');
  assert.deepEqual(calls[0]!.args, [
    'delegate',
    'preview',
    '--from',
    'main',
    '--to',
    'HEAD',
    '--repo',
    workspace,
    '--rule',
    rule,
  ]);
  assert.equal(calls[0]!.env.OCR_NO_UPDATE, '1');
  assert.equal(calls[0]!.env.GITHUB_TOKEN, undefined);
  assert.equal(calls[0]!.env.OCR_LLM_TOKEN, undefined);
  assert.equal(result.action, 'preview');
});

test('preview: commit mode and workspace mode', async () => {
  const { workspace } = fixture();
  const { deps, calls } = fakeDeps(workspace);

  await executeOcr({ action: 'preview', commit: 'abc1234' }, deps);
  assert.deepEqual(calls[0]!.args.slice(0, 5), ['delegate', 'preview', '--commit', 'abc1234', '--repo']);
  assert.equal(calls[0]!.args[5], workspace);

  await executeOcr({ action: 'preview' }, deps);
  assert.deepEqual(calls[1]!.args.slice(0, 3), ['delegate', 'preview', '--repo']);
});

test('rule: puts flags before paths', async () => {
  const { workspace, rule } = fixture();
  const { deps, calls } = fakeDeps(workspace, RULES);

  const result = await executeOcr(
    { action: 'rule', paths: ['src/a.rs', 'src/b.rs'] },
    deps,
  );

  assert.deepEqual(calls[0]!.args, [
    'delegate',
    'rule',
    '--repo',
    workspace,
    '--rule',
    rule,
    'src/a.rs',
    'src/b.rs',
  ]);
  assert.equal(result.action, 'rule');
  if (result.action === 'rule') {
    assert.equal(result.groups.length, 2);
    assert.deepEqual(result.groups[0]!.appliesTo, ['src/a.dart']);
    assert.equal(result.groups[0]!.content, 'Be correct.');
    assert.equal(result.groups[1]!.content, 'No unwrap.');
  }
});

test('defaults: the rule file is the workspace copy', async () => {
  const { workspace, rule } = fixture(false, true);
  const { deps, calls } = fakeDeps(workspace);

  await executeOcr({ action: 'preview', from: 'main', to: 'HEAD' }, deps);

  assert.equal(calls[0]!.args[calls[0]!.args.indexOf('--repo') + 1], workspace);
  assert.equal(calls[0]!.args[calls[0]!.args.indexOf('--rule') + 1], rule);
});

test('defaults: a missing trusted rule fails instead of using the PR copy', () => {
  const { workspace } = fixture(false, false);
  const { deps } = fakeDeps(workspace);

  assert.throws(
    () => buildOcrArgs({ action: 'preview', from: 'main', to: 'HEAD' }, deps),
    /no trusted review rules/,
  );
});

test('rejects unsafe or incomplete arguments', () => {
  const { workspace } = fixture();
  const { deps } = fakeDeps(workspace);
  const bad: OcrParams[] = [
    { action: 'preview', from: '--evil', to: 'HEAD' },
    { action: 'preview', from: 'main' },
    { action: 'preview', from: 'main', to: 'HEAD', commit: 'abc1234' },
    { action: 'preview', commit: 'not-a-sha' },
    { action: 'rule', paths: [] },
    { action: 'rule', paths: ['-x'] },
    { action: 'rule', paths: ['../x'] },
    { action: 'rule', paths: ['/tmp/x'] },
    { action: 'preview', repo: '../outside' },
    { action: 'preview', rule: '/etc/passwd' },
  ];
  for (const params of bad) {
    assert.throws(() => buildOcrArgs(params, deps), Error, JSON.stringify(params));
  }
});

test('preview parser: files, exclusions, and metadata', () => {
  const parsed = parsePreview(PREVIEW);
  assert.equal(parsed.mode, 'range');
  assert.equal(parsed.reviewable, 2);
  assert.equal(parsed.total, 4);
  assert.deepEqual(parsed.files, [
    { path: 'src/a.rs', status: 'modified', insertions: 6, deletions: 2 },
    { path: 'src/b.rs', status: 'added', insertions: 4, deletions: 0 },
  ]);
  assert.deepEqual(parsed.excluded, [
    { path: 'src/deleted.rs', status: 'deleted', insertions: 0, deletions: 9, reason: 'deleted' },
    { path: 'src/gen.rs', status: 'modified', insertions: 1, deletions: 1, reason: 'user_exclude' },
  ]);
  assert.equal(parsed.merge_base, 'abc123');
  assert.equal(parsed.total_insertions, 10);
});

test('preview parser: rejects unexpected output', () => {
  assert.throws(() => parsePreview('not a preview'), /missing "# Files/);
  assert.throws(
    () => parsePreview('# Files (2 reviewable / 2 total)\n\n- mode: range\n'),
    /header lists 2 reviewable files but parsed 0/,
  );
});

test('rule parser: rejects output without groups or empty content', () => {
  assert.throws(() => parseRules('not rules'), /no rule groups/);
  assert.throws(
    () =>
      parseRules(
        '### Rule Group 1: system / default\n\nApplies to:\n- src/a.rs\n\n#### Content\n\n---\n',
      ),
    /empty content/,
  );
});

test('execute: missing binary and non-zero exit surface actionable errors', async () => {
  const { workspace } = fixture();
  const { deps } = fakeDeps(workspace);
  deps.run = async () => {
    throw Object.assign(new Error('spawn ocr ENOENT'), { code: 'ENOENT' });
  };
  await assert.rejects(
    executeOcr({ action: 'preview', from: 'main', to: 'HEAD' }, deps),
    /ocr not found: install/,
  );

  deps.run = async () => {
    throw Object.assign(new Error('exit 1'), { code: 1, stderr: 'bad ref\n' });
  };
  await assert.rejects(
    executeOcr({ action: 'preview', from: 'main', to: 'HEAD' }, deps),
    /ocr preview failed \(1\): bad ref/,
  );
});

test('tools: the extension registers the ocr and codegraph tools', () => {
  const names: string[] = [];
  const pi = {
    registerTool: (tool: { name: string }) => {
      names.push(tool.name);
    },
  } as unknown as Parameters<typeof registerTools>[0];
  registerTools(pi);
  assert.deepEqual(names, ['ocr', 'codegraph']);
});

test('a rule path outside the workspace is rejected', () => {
  const { workspace } = fixture();
  const { deps } = fakeDeps(workspace);

  assert.throws(
    () =>
      buildOcrArgs(
        { action: 'preview', from: 'main', to: 'HEAD', rule: '../outside-rule.json' },
        deps,
      ),
    /must stay inside the workspace/,
  );
});

test('ref validation accepts underscore- and plus-prefixed refs', () => {
  const { workspace } = fixture();
  const { deps } = fakeDeps(workspace);

  assert.doesNotThrow(() =>
    buildOcrArgs({ action: 'preview', from: '_main', to: 'HEAD' }, deps),
  );
  assert.doesNotThrow(() =>
    buildOcrArgs({ action: 'preview', from: '+ref', to: 'HEAD' }, deps),
  );
});

test('the rule file is pinned by OCR_RULE_FILE', () => {
  const { workspace, rule } = fixture();
  const { deps } = fakeDeps(workspace);
  deps.env = { ...deps.env, OCR_RULE_FILE: rule };

  const { args, rule: resolved } = buildOcrArgs(
    { action: 'preview', from: 'main', to: 'HEAD' },
    deps,
  );
  assert.equal(args[args.indexOf('--rule') + 1], rule);
  assert.equal(resolved, rule);

  // A model that passes its own rule file cannot override the pin.
  const overridden = buildOcrArgs(
    { action: 'preview', from: 'main', to: 'HEAD', rule: '.opencodereview/rule.json' },
    deps,
  );
  assert.equal(overridden.args[overridden.args.indexOf('--rule') + 1], rule);
});

test('OCR_RULE_FILE outside the workspace is rejected', () => {
  const { workspace } = fixture();
  const { deps } = fakeDeps(workspace);
  deps.env = { ...deps.env, OCR_RULE_FILE: '/etc/passwd' };

  assert.throws(
    () => buildOcrArgs({ action: 'preview', from: 'main', to: 'HEAD' }, deps),
    /must be inside the workspace/,
  );
});

test('the preview parser reconciles the excluded count against the total', () => {
  const parsed = parsePreview(PREVIEW);
  assert.equal(parsed.files.length + parsed.excluded.length, parsed.total);

  // A dropped exclusion line is a parse failure, not a silent gap.
  assert.throws(
    () =>
      parsePreview(
        '# Files (1 reviewable / 3 total)\n\n- mode: range\n\n  - `src/a.rs` [modified] +1/-0\n',
      ),
    /header lists 3 files but parsed 1 reviewable and 0 excluded/,
  );
});

test('the real command runner executes', async () => {
  const result = await execFileText(
    process.execPath,
    ['-e', 'process.stdout.write("ok")'],
    { cwd: process.cwd(), env: process.env, timeout: 10_000, maxBuffer: 1024, shell: false },
  );
  assert.equal(result.stdout, 'ok');
});
