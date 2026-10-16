import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CONFIDENCE_FLOOR } from './src/constants.ts';
import { MAX_BATCH_SIZE, chunk, parseBatchSize, parseReviewConfig } from './src/config.ts';
import { clampToAnchor, isContiguous, matchesGlob, parseDiffAnchors } from './src/diff.ts';
import { splitReviewable } from './src/files.ts';
import { buildReviewComments, parseFindings, splitByConfidence } from './src/findings.ts';
import type { DiffAnchors, ReviewFinding } from './src/types.ts';

function finding(overrides: Partial<ReviewFinding> = {}): ReviewFinding {
  return {
    path: 'src/a.ts',
    start_line: 14,
    severity: 'high',
    category: 'bug',
    title: 't',
    body: 'b',
    confidence: 0.9,
    ...overrides,
  };
}

const anchors: DiffAnchors = new Map([['src/a.ts', new Set([10, 14, 30])]]);

test('glob: ** crosses directories and **/ also matches root-level files', () => {
  assert.ok(matchesGlob('src/a/b/c.ts', 'src/**/*.ts'));
  assert.ok(matchesGlob('src/c.ts', 'src/**/*.ts'));
  assert.ok(matchesGlob('c.ts', '**/*.ts'));
  assert.ok(!matchesGlob('src/c.md', 'src/**/*.ts'));
  assert.ok(matchesGlob('crates/codec/src/lib.rs', 'crates/{codec,resampler}/**/*.rs'));
  assert.ok(matchesGlob('crates/resampler/x.rs', 'crates/{codec,resampler}/**/*.rs'));
  assert.ok(!matchesGlob('crates/stretch/x.rs', 'crates/{codec,resampler}/**/*.rs'));
});

test('glob: unterminated brace does not throw and does not match', () => {
  assert.doesNotThrow(() => matchesGlob('src/a.ts', 'src/{a,b'));
  assert.ok(!matchesGlob('src/a.ts', 'src/{a,b'));
});

test('glob: * and ? do not cross directory separators', () => {
  assert.ok(!matchesGlob('a/b/c.ts', 'a/*.ts'));
  assert.ok(matchesGlob('a/x.ts', 'a/?.ts'));
  assert.ok(!matchesGlob('a/xy.ts', 'a/?.ts'));
});

test('glob: slash-less patterns also match the basename; slashed patterns do not', () => {
  assert.ok(matchesGlob('packages/app/yarn.lock', 'yarn.lock'));
  assert.ok(matchesGlob('deep/dir/x.ts', '*.ts'));
  assert.ok(!matchesGlob('deep/dir/x/Cargo.lock', 'dist/Cargo.lock'));
});

test('glob: dotfiles match wildcards and regex metacharacters are literal', () => {
  assert.ok(matchesGlob('.github/x.yml', '*.yml'));
  assert.ok(matchesGlob('.cursor/rules/a.mdc', '**/*.mdc'));
  assert.ok(matchesGlob('a+b.ts', 'a+b.ts'));
  assert.ok(!matchesGlob('aab.ts', 'a+b.ts'));
});

test('diff anchors: context and added lines are anchors; removed lines are not counted', () => {
  const patch = [
    '@@ -1,4 +1,4 @@',
    ' keep1',
    '-old',
    '+new',
    ' keep2',
    ' keep3',
  ].join('\n');
  const lines = parseDiffAnchors([{ path: 'f.ts', patch }]).get('f.ts')!;
  assert.deepEqual([...lines].sort((a, b) => a - b), [1, 2, 3, 4]);
});

test('diff anchors: "\\ No newline" markers are ignored and each hunk resets the counter', () => {
  const patch = [
    '@@ -1,2 +1,2 @@',
    ' a',
    '-b',
    '\\ No newline at end of file',
    '+b2',
    '@@ -10,1 +20,2 @@',
    ' x',
    '+y',
  ].join('\n');
  const lines = parseDiffAnchors([{ path: 'f.ts', patch }]).get('f.ts')!;
  assert.deepEqual([...lines].sort((a, b) => a - b), [1, 2, 20, 21]);
});

test('diff anchors: a patch with no hunk header contributes no line 0', () => {
  const lines = parseDiffAnchors([{ path: 'f.ts', patch: '+orphan\n line' }]).get('f.ts')!;
  assert.equal(lines.size, 0);
});

test('clamp: exact hit, nearest within tolerance, beyond tolerance, tie', () => {
  assert.equal(clampToAnchor(anchors, 'src/a.ts', 14), 14);
  assert.equal(clampToAnchor(anchors, 'src/a.ts', 13), 14);
  assert.equal(clampToAnchor(anchors, 'src/a.ts', 33), 30);
  assert.equal(clampToAnchor(anchors, 'src/a.ts', 34), null);
  assert.equal(clampToAnchor(anchors, 'src/a.ts', 12), 10);
  assert.equal(clampToAnchor(anchors, 'other.ts', 14), null);
});

test('start_line: a single-line finding never carries start_line', () => {
  const { comments } = buildReviewComments([finding({ start_line: 11 })], anchors);
  assert.equal(comments.length, 1);
  assert.equal(comments[0]!.line, 10);
  assert.equal('start_line' in comments[0]!, false);
});

test('start_line: multi-line finding with unclamped start falls back to the clamped end', () => {
  const { comments } = buildReviewComments(
    [finding({ start_line: 40, end_line: 30 })],
    anchors,
  );
  assert.equal(comments[0]!.line, 30);
  assert.equal('start_line' in comments[0]!, false);
});

test('start_line: a contiguous multi-line range keeps its clamped start line', () => {
  const contig: DiffAnchors = new Map([['src/a.ts', new Set([10, 11, 12, 13, 14])]]);
  const { comments } = buildReviewComments(
    [finding({ start_line: 11, end_line: 14 })],
    contig,
  );
  assert.equal(comments[0]!.line, 14);
  assert.equal(comments[0]!.start_line, 11);
  assert.equal(comments[0]!.start_side, 'RIGHT');
});

test('start_line: a range spanning separate diff runs collapses to one line', () => {
  // 14 and 30 are anchors, but 15..29 are not in the diff: GitHub would 422.
  const { comments } = buildReviewComments(
    [finding({ start_line: 13, end_line: 30 })],
    anchors,
  );
  assert.equal(comments[0]!.line, 30);
  assert.equal('start_line' in comments[0]!, false);
});

test('start_line: crossed anchors collapse instead of dropping the finding', () => {
  // end 12 snaps down to 10, start 16 snaps up to 14: the anchors cross.
  const { comments } = buildReviewComments(
    [finding({ start_line: 16, end_line: 12 })],
    anchors,
  );
  assert.equal(comments[0]!.line, 14);
  assert.equal('start_line' in comments[0]!, false);
});

test('contiguity: only fully-anchored ranges count as contiguous', () => {
  const set = new Set([1, 2, 3]);
  assert.equal(isContiguous(set, 1, 3), true);
  assert.equal(isContiguous(set, 1, 2), true);
  assert.equal(isContiguous(set, 1, 4), false);
  assert.equal(isContiguous(set, 2, 2), true);
});

test('confidence floor: 0.55 is kept, 0.549 is dropped', () => {
  assert.equal(CONFIDENCE_FLOOR, 0.55);
  const { kept, dropped } = splitByConfidence([
    finding({ confidence: 0.55 }),
    finding({ confidence: 0.549 }),
  ]);
  assert.deepEqual(kept.map((f) => f.confidence), [0.55]);
  assert.equal(dropped, 1);
});

test('batching: 101 comments at size 50 split into 50, 50, 1', () => {
  const batches = chunk(
    Array.from({ length: 101 }, (_, i) => i),
    50,
  );
  assert.deepEqual(batches.map((b) => b.length), [50, 50, 1]);
});

test('config: null yields defaults; malformed arrays and entries are ignored', () => {
  assert.deepEqual(parseReviewConfig(null), { exclude: [], rules: [] });
  assert.deepEqual(
    parseReviewConfig('{"exclude":"dist/**","rules":{"path":"x"}}'),
    { exclude: [], rules: [] },
  );
  assert.deepEqual(
    parseReviewConfig(
      '{"exclude":["a/**",3],"rules":[{"path":"x/**","rule":"r"},{"path":1,"rule":"bad"}]}',
    ),
    { exclude: ['a/**'], rules: [{ path: 'x/**', rule: 'r' }] },
  );
});

test('batch size: clamped to the maximum, defaults when invalid', () => {
  assert.equal(parseBatchSize(undefined), 50);
  assert.equal(parseBatchSize(''), 50);
  assert.equal(parseBatchSize('0'), 50);
  assert.equal(parseBatchSize('-4'), 50);
  assert.equal(parseBatchSize('abc'), 50);
  assert.equal(parseBatchSize('1'), 1);
  assert.equal(parseBatchSize('25'), 25);
  assert.equal(parseBatchSize(String(MAX_BATCH_SIZE)), MAX_BATCH_SIZE);
  assert.equal(parseBatchSize('100000'), MAX_BATCH_SIZE);
});

test('reviewable files: removed and excluded paths are dropped for anchoring too', () => {
  const files = [
    { filename: 'src/a.ts', status: 'modified' },
    { filename: 'src/gone.ts', status: 'removed' },
    { filename: 'Cargo.lock', status: 'modified' },
    { filename: 'apps/gui-flutter/build/x.dart', status: 'added' },
  ];
  const config = { exclude: ['**/Cargo.lock', 'apps/gui-flutter/build/**'], rules: [] };
  const { kept, skippedExcluded } = splitReviewable(files, config);
  assert.deepEqual(
    kept.map((f) => f.filename),
    ['src/a.ts'],
  );
  assert.deepEqual(skippedExcluded, ['Cargo.lock', 'apps/gui-flutter/build/x.dart']);
});

test('findings: NaN confidence is rejected instead of silently dropped later', () => {
  const base = finding();
  assert.equal(parseFindings([base]).length, 1);
  assert.throws(() => parseFindings([{ ...base, confidence: Number.NaN }]), /confidence/);
  assert.throws(() => parseFindings([{ ...base, confidence: Infinity }]), /confidence/);
  assert.throws(() => parseFindings([{ ...base, confidence: '0.9' }]), /confidence/);
  assert.throws(() => parseFindings([{ ...base, path: '' }]), /path/);
  assert.throws(() => parseFindings([{ ...base, severity: 'urgent' }]), /severity/);
  assert.throws(() => parseFindings('nope'), /array/);
});

test('batch size: strict parsing rejects trailing junk instead of truncating', () => {
  // parseInt would return 25 here; strict parsing must fall back to the default.
  assert.equal(parseBatchSize('25abc'), 50);
  assert.equal(parseBatchSize(' 25 '), 25);
  assert.equal(parseBatchSize('25.5'), 50);
  // parseInt would return 1; strict reads 1000 and clamps.
  assert.equal(parseBatchSize('1e3'), MAX_BATCH_SIZE);
  assert.equal(parseBatchSize('0x10'), 16);
});

test('chunk: a non-positive size throws instead of looping forever', () => {
  assert.throws(() => chunk([1, 2, 3], 0), /positive integer/);
  assert.throws(() => chunk([1, 2, 3], -1), /positive integer/);
  assert.throws(() => chunk([1, 2, 3], 1.5), /positive integer/);
  assert.deepEqual(chunk([1, 2, 3], 2), [[1, 2], [3]]);
});

test('findings: a null end_line normalizes to undefined', () => {
  const parsed = parseFindings([{ ...finding(), end_line: null }]);
  assert.equal(parsed[0]!.end_line, undefined);
  assert.equal('end_line' in parsed[0]!, true);
});
