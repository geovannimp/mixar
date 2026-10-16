import { test } from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { loadRuntime, resolvePrNumber } from './src/runtime.ts';

test('pr number: event payload is used when no override is set', () => {
  assert.equal(resolvePrNumber({}, { pull_request: { number: 7 } }), 7);
  assert.equal(resolvePrNumber({}, { issue: { number: 9 } }), 9);
  // pull_request wins over issue when both are present.
  assert.equal(resolvePrNumber({}, { pull_request: { number: 7 }, issue: { number: 9 } }), 7);
});

test('pr number: override wins, and invalid overrides fail loudly', () => {
  assert.equal(resolvePrNumber({ PI_REVIEW_PR_NUMBER: '42' }, {}), 42);
  assert.equal(resolvePrNumber({ PI_REVIEW_PR_NUMBER: ' 42 ' }, {}), 42);
  assert.equal(resolvePrNumber({ PI_REVIEW_PR_NUMBER: '42' }, { pull_request: { number: 7 } }), 42);
  assert.throws(() => resolvePrNumber({ PI_REVIEW_PR_NUMBER: 'abc' }, {}), /not a valid PR number/);
  assert.throws(() => resolvePrNumber({ PI_REVIEW_PR_NUMBER: '0' }, {}), /not a valid PR number/);
  assert.throws(() => resolvePrNumber({ PI_REVIEW_PR_NUMBER: '-3' }, {}), /not a valid PR number/);
  // A whitespace-only override is treated as absent, not as a value.
  assert.equal(resolvePrNumber({ PI_REVIEW_PR_NUMBER: '   ' }, { pull_request: { number: 7 } }), 7);
});

test('pr number: no override and no payload is an error', () => {
  assert.throws(() => resolvePrNumber({}, {}), /cannot resolve PR number/);
});

test('runtime: token and repository are required', () => {
  assert.throws(() => loadRuntime({}), /GITHUB_TOKEN is required/);
  assert.throws(
    () => loadRuntime({ GITHUB_TOKEN: 't', GITHUB_REPOSITORY: 'owner' }),
    /GITHUB_REPOSITORY must be owner\/repo/,
  );
  assert.throws(
    () => loadRuntime({ GITHUB_TOKEN: 't', GITHUB_REPOSITORY: 'a/b/c' }),
    /GITHUB_REPOSITORY must be owner\/repo/,
  );
});

test('runtime: reads the event payload only when there is no override', () => {
  const dir = mkdtempSync(join(tmpdir(), 'pi-review-'));
  const eventPath = join(dir, 'event.json');
  writeFileSync(eventPath, JSON.stringify({ pull_request: { number: 12 } }));

  const base = {
    GITHUB_TOKEN: 't',
    GITHUB_REPOSITORY: 'o/r',
    GITHUB_EVENT_PATH: eventPath,
  };
  assert.equal(loadRuntime(base).prNumber, 12);
  assert.equal(loadRuntime({ ...base, PI_REVIEW_PR_NUMBER: '5' }).prNumber, 5);
  // The whitespace-only override must still fall through to the event payload,
  // rather than skipping the file and failing to find a PR.
  assert.equal(loadRuntime({ ...base, PI_REVIEW_PR_NUMBER: '   ' }).prNumber, 12);
  assert.equal(loadRuntime(base).dryRun, false);
  assert.equal(loadRuntime({ ...base, PI_REVIEW_DRY_RUN: '1' }).dryRun, true);
});

test('runtime: an unreadable event payload names the path', () => {
  assert.throws(
    () =>
      loadRuntime({
        GITHUB_TOKEN: 't',
        GITHUB_REPOSITORY: 'o/r',
        GITHUB_EVENT_PATH: '/nonexistent/event.json',
      }),
    /Failed to read GitHub event payload from \/nonexistent\/event\.json/,
  );
});
