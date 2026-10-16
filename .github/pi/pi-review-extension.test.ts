import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  CONFIDENCE_FLOOR,
  type DiffAnchors,
  type ReviewFinding,
  buildReviewComments,
  chunk,
  clampToAnchor,
  matchesGlob,
  parseDiffAnchors,
  parseReviewConfig,
  splitByConfidence,
} from './pi-review-extension.ts';

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

test('start_line: multi-line finding keeps a clamped start line when it is above the end', () => {
  const { comments } = buildReviewComments(
    [finding({ start_line: 13, end_line: 30 })],
    anchors,
  );
  assert.equal(comments[0]!.line, 30);
  assert.equal(comments[0]!.start_line, 14);
  assert.equal(comments[0]!.start_side, 'RIGHT');
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
