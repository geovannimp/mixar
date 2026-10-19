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

interface Processes {
  /** pid → parent pid */
  parent: Map<number, number>;
  /** pid → command line, NUL-separated as /proc reports it */
  cmdline: Map<number, string>;
}

function readProcesses(reader: ProcReader): Processes {
  const parent = new Map<number, number>();
  const cmdline = new Map<number, string>();
  for (const entry of reader.list()) {
    if (!/^\d+$/.test(entry)) continue;
    const pid = Number(entry);
    try {
      parent.set(pid, Number(/^PPid:\s+(\d+)$/m.exec(reader.read(`/proc/${entry}/status`))?.[1]));
      cmdline.set(pid, reader.read(`/proc/${entry}/cmdline`));
    } catch {
      // Not a readable process entry (it exited, or it is another user's).
      parent.delete(pid);
      cmdline.delete(pid);
    }
  }
  return { parent, cmdline };
}

/**
 * Every process of every CodeGraph MCP server this host started, parents first.
 *
 * The server is a stdio child of the process that started the session, and that
 * shim is the handle pi's client holds. The shim runs the platform binary, which
 * runs a watchdog, so the tree is terminated explicitly — leaves first — instead
 * of trusting those grandchildren to notice the shim is gone. Only the direct
 * child is matched by command line; the rest follow from parentage.
 */
export function mcpServerPids(selfPid: number, reader: ProcReader = procReader): number[] {
  const { parent, cmdline } = readProcesses(reader);
  const roots = [...parent]
    .filter(
      ([pid, pidParent]) =>
        pidParent === selfPid && MCP_SERVER_TOKENS.every((t) => cmdline.get(pid)?.includes(t)),
    )
    .map(([pid]) => pid);

  const tree: number[] = [];
  let frontier = roots;
  while (frontier.length > 0) {
    tree.push(...frontier);
    const parents = new Set(frontier);
    frontier = [...parent].filter(([, pidParent]) => parents.has(pidParent)).map(([pid]) => pid);
  }
  return tree;
}

/** SIGTERM every CodeGraph MCP server process this host started; returns the pids signalled. */
export function terminateMcpServers(selfPid: number = process.pid): number[] {
  const terminated: number[] = [];
  // Leaves first: killing a parent reparents its children, and a reparented
  // process is no longer reached by pid from a stale pid list.
  for (const pid of mcpServerPids(selfPid).reverse()) {
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
    const pids = terminateServers();
    // The step log is the only place the teardown shows up, and a run that hangs
    // because a server survived has to be distinguishable from one that never
    // reached the teardown. Written to stderr directly: extension console output
    // does not reach the step log, this does.
    process.stderr.write(`[mcp-teardown] ended ${pids.length} codegraph process(es)\n`);
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
