import { test } from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as pause } from 'node:timers/promises';

import { loadRuntime, resolvePrNumber } from './src/runtime.ts';
import { assertCommentOnPullRequest } from './src/threads.ts';
import { mcpServerPids, registerMcpTeardown } from './src/mcp-teardown.ts';
import type { Runtime } from './src/runtime.ts';

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

test('runtime: padded GITHUB_REPOSITORY segments are rejected, not silently used', () => {
  const base = { GITHUB_TOKEN: 't' };
  assert.throws(
    () => loadRuntime({ ...base, GITHUB_REPOSITORY: 'owner/ ' }),
    /GITHUB_REPOSITORY must be owner\/repo/,
  );
  assert.throws(
    () => loadRuntime({ ...base, GITHUB_REPOSITORY: ' /repo' }),
    /GITHUB_REPOSITORY must be owner\/repo/,
  );
  // Surrounding whitespace around the whole value is still accepted.
  assert.equal(loadRuntime({ ...base, GITHUB_REPOSITORY: ' o/r ', PI_REVIEW_PR_NUMBER: '3' }).repo, 'r');
});

test('runtime: the process environment is cached, explicit env objects are not', () => {
  const a = { GITHUB_TOKEN: 't', GITHUB_REPOSITORY: 'o/r', PI_REVIEW_PR_NUMBER: '1' };
  const b = { ...a, PI_REVIEW_PR_NUMBER: '2' };
  assert.equal(loadRuntime(a).prNumber, 1);
  assert.equal(loadRuntime(b).prNumber, 2, 'explicit envs must not be served from the cache');
});

test('comment ownership: only comments on the PR under review are accepted', () => {
  const rt = { prNumber: 23 } as Runtime;
  const url = (n: number) => `https://api.github.com/repos/o/r/pulls/${n}`;

  assert.doesNotThrow(() => assertCommentOnPullRequest(rt, { pull_request_url: url(23) }));
  // An adjacent number is a different PR, not a match.
  assert.throws(
    () => assertCommentOnPullRequest(rt, { pull_request_url: url(123) }),
    /this run reviews PR #23/,
  );
  assert.throws(() => assertCommentOnPullRequest(rt, { pull_request_url: url(24) }));
  assert.throws(() => assertCommentOnPullRequest(rt, {}), /an unknown pull request/);
});

/**
 * The MCP server pi spawns is a child of this process; the teardown must find it
 * and the processes it starts, or those are left for the runner to reap.
 */
test('mcp teardown: the whole codegraph serve subtree of this process is selected', () => {
  const files: Record<string, string> = {
    '/proc/11/status': 'Name:\tnode\nPPid:\t100\n',
    '/proc/11/cmdline': 'node\0/usr/lib/node_modules/codegraph/bin/codegraph\0serve\0--mcp\0--no-watch\0',
    // The platform binary 11 runs, and the watchdog 21 runs: neither names the
    // tokens on its own, and both must go with the shim.
    '/proc/21/status': 'Name:\tnode\nPPid:\t11\n',
    '/proc/21/cmdline': 'node\0--liftoff-only\0codegraph.js\0serve\0--mcp\0',
    '/proc/22/status': 'Name:\tnode\nPPid:\t21\n',
    '/proc/22/cmdline': 'node\0-e\0const parentPid = Number(process.argv[1]);\0',
    // Right command, different parent: not ours to kill.
    '/proc/12/status': 'Name:\tnode\nPPid:\t1\n',
    '/proc/12/cmdline': 'codegraph\0serve\0--mcp\0',
    // Our children, but not the MCP server.
    '/proc/13/status': 'Name:\tgit\nPPid:\t100\n',
    '/proc/13/cmdline': 'git\0status\0',
    '/proc/14/status': 'Name:\tnode\nPPid:\t100\n',
    '/proc/14/cmdline': 'codegraph\0status\0pr-head\0',
  };
  const reader = {
    list: () => ['memory', 'cgroup', '11', '12', '13', '14', '21', '22', '15'],
    read: (path: string) => {
      const content = files[path];
      // /proc/15 vanished between listing and reading, as processes do.
      if (content === undefined) throw new Error(`ENOENT: ${path}`);
      return content;
    },
  };

  // Parents first: terminateMcpServers reverses this to signal leaves first.
  assert.deepEqual(mcpServerPids(100, reader), [11, 21, 22]);
  assert.deepEqual(mcpServerPids(1, reader), [12]);
  assert.deepEqual(mcpServerPids(999, reader), []);
});

/** The run has to end the server itself; agent_end must not outlive a retry. */
test('mcp teardown: settled tears down, agent_end defers, agent_start cancels', async () => {
  const registered: Record<string, (() => void)[]> = {};
  const pi = {
    on: (event: string, handler: () => void) => {
      registered[event] = [...(registered[event] ?? []), handler];
      return () => {};
    },
  } as unknown as Parameters<typeof registerMcpTeardown>[0];
  const fire = (event: string) => {
    for (const handler of registered[event] ?? []) handler();
  };

  let terminated = 0;
  registerMcpTeardown(pi, { graceMs: 5, terminate: () => ((terminated += 1), []) });

  fire('agent_end');
  assert.equal(terminated, 0, 'agent_end must wait for a possible retry');
  await pause(30);
  assert.equal(terminated, 1, 'a run that ends without settling must still exit');

  fire('agent_end');
  fire('agent_start');
  await pause(30);
  assert.equal(terminated, 1, 'agent_start must cancel the pending teardown');

  // agent_settled is the last word: it tears down now, cancelling any timer.
  fire('agent_end');
  fire('agent_settled');
  assert.equal(terminated, 2);
  await pause(30);
  assert.equal(terminated, 2, 'settling must not queue a second teardown');
});
