import { readdirSync, readFileSync } from 'node:fs';
import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';

/**
 * Terms the CodeGraph MCP server appears with in its command line, so the scan
 * below cannot match an unrelated child of this process.
 */
const MCP_SERVER_TOKENS = ['codegraph', 'serve'];

/** How long `agent_end` waits for a retry or a fresh turn before tearing down. */
export const MCP_TEARDOWN_GRACE_MS = 30_000;

interface ProcReader {
  list(): string[];
  read(path: string): string;
}

const procReader: ProcReader = {
  list: () => readdirSync('/proc'),
  read: (path) => readFileSync(path, 'utf8'),
};

/**
 * Direct children of `selfPid` that look like the CodeGraph MCP server.
 *
 * MCP servers are stdio children of the process that started the session, so
 * "child of this process whose command line mentions codegraph serve" is exactly
 * the process that has to be terminated. Any unreadable entry is skipped: the
 * process most likely exited between listing and reading.
 */
export function mcpServerChildPids(selfPid: number, reader: ProcReader = procReader): number[] {
  const pids: number[] = [];
  for (const entry of reader.list()) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const parent = Number(/^PPid:\s+(\d+)$/m.exec(reader.read(`/proc/${entry}/status`))?.[1]);
      if (parent !== selfPid) continue;
      const cmdline = reader.read(`/proc/${entry}/cmdline`);
      if (MCP_SERVER_TOKENS.every((token) => cmdline.includes(token))) pids.push(Number(entry));
    } catch {
      // Not a readable process entry: nothing to terminate.
    }
  }
  return pids;
}

/** SIGTERM every CodeGraph MCP server this process started; returns the pids signalled. */
export function terminateMcpServers(selfPid: number = process.pid): number[] {
  const terminated: number[] = [];
  for (const pid of mcpServerChildPids(selfPid)) {
    try {
      process.kill(pid, 'SIGTERM');
      terminated.push(pid);
    } catch {
      // Already gone.
    }
  }
  return terminated;
}

export interface McpTeardownOptions {
  /** How long `agent_end` waits for a retry or a fresh turn before tearing down. */
  graceMs?: number;
  /** Test seam: what runs the teardown. Defaults to terminating the MCP children. */
  terminate?: () => number[];
}

/**
 * End the CodeGraph MCP server when the run is over, so the action process can exit.
 *
 * Upstream teardown gap (`shaftoe/pi-coding-agent-action` v2.30.0 with pi 1.0.4):
 * pi spawns stdio MCP servers as child processes and closes them only from its
 * `session_shutdown` extension event, which the SDK path this action drives never
 * emits — `AgentSession.dispose()` does not emit it — and the action never calls
 * `process.exit()`. A live MCP child therefore keeps Node's event loop alive after
 * the review is posted, the `Pi review` step never completes, and the job is
 * cancelled at its 45-minute timeout with the review already on the PR. The child
 * cannot clear itself either: it exits on stdin EOF, and the parent holds that pipe
 * open, so the session has to end it.
 *
 * `agent_settled` is the normal trigger: it fires once, after no retry, compaction
 * or queued continuation is left to run. `agent_end` arms a grace timer so a run
 * that ends any other way (provider error, abort) also exits, and `agent_start` —
 * a retry or a fresh turn — cancels it, which keeps CodeGraph available for the
 * rest of the run. Termination is idempotent, and a server pi reconnects later is
 * ended by the next trigger.
 */
export function registerMcpTeardown(pi: ExtensionAPI, options: McpTeardownOptions = {}): void {
  const graceMs = options.graceMs ?? MCP_TEARDOWN_GRACE_MS;
  const terminateServers = options.terminate ?? terminateMcpServers;
  let pending: NodeJS.Timeout | undefined;

  const cancel = (): void => {
    if (pending !== undefined) {
      clearTimeout(pending);
      pending = undefined;
    }
  };

  const terminate = (): void => {
    cancel();
    terminateServers();
  };

  pi.on('agent_start', () => cancel());
  pi.on('agent_settled', () => terminate());
  pi.on('agent_end', () => {
    cancel();
    pending = setTimeout(terminate, graceMs);
    // Never let the timer itself hold the event loop open: the leaked MCP child
    // already does, and ending that is what the timer exists for.
    pending.unref();
  });
}
