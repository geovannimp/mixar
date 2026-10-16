import { execFileSync } from 'node:child_process';
import { Agent, type SDKAgent, type McpServerConfig } from '@cursor/sdk';

export interface AgentFactoryOptions {
  apiKey: string;
  model: string;
  cwd: string;
  name: string;
  systemPrompt: string;
  /** Empty array = text-only (intent classifier). */
  tools?: Array<'read' | 'grep' | 'glob' | 'ls' | 'mcp'>;
  enableCodegraphMcp?: boolean;
}

export interface IndexResult {
  ok: boolean;
  degraded: boolean;
  message: string;
}

/** Host-side CodeGraph index before agent runs (shell is disallowed to agent). */
export function ensureCodegraphIndex(cwd: string): IndexResult {
  try {
    try {
      execFileSync('codegraph', ['status'], {
        cwd,
        stdio: ['ignore', 'pipe', 'pipe'],
        encoding: 'utf8',
        timeout: 60_000,
      });
    } catch {
      execFileSync('codegraph', ['init'], {
        cwd,
        stdio: ['ignore', 'pipe', 'pipe'],
        encoding: 'utf8',
        timeout: 120_000,
      });
    }
    execFileSync('codegraph', ['index'], {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
      timeout: 600_000,
    });
    return { ok: true, degraded: false, message: 'CodeGraph indexed' };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(`CodeGraph index failed; continuing degraded: ${message}`);
    return {
      ok: false,
      degraded: true,
      message: `CodeGraph index failed: ${message}`,
    };
  }
}

function codegraphMcpConfig(cwd: string): McpServerConfig {
  return {
    type: 'stdio',
    command: 'codegraph',
    args: ['serve', '--mcp'],
    cwd,
    env: {
      CODEGRAPH_MCP_TOOLS:
        process.env.CODEGRAPH_MCP_TOOLS ??
        'explore,node,search,callers,callees,impact,files,status',
    },
  };
}

/**
 * Create a local Cursor agent.
 * Ruling: tools must include `mcp` when CodeGraph MCP is enabled — SDK disables
 * MCP entirely if `mcp` is omitted from an allowlist.
 * Ruling: disallowedTools omits invalid `write` (not a ToolName); use edit/delete/shell.
 */
export async function createReviewAgent(
  opts: AgentFactoryOptions,
): Promise<SDKAgent> {
  const tools = opts.tools ?? ['read', 'grep', 'glob', 'ls', 'mcp'];
  const mcpServers =
    opts.enableCodegraphMcp === false
      ? undefined
      : { codegraph: codegraphMcpConfig(opts.cwd) };

  return Agent.create({
    apiKey: opts.apiKey,
    model: { id: opts.model },
    name: opts.name,
    systemPrompt: opts.systemPrompt,
    tools,
    // `write` is not a ToolName — edit covers mutations. Deny shell/edit/delete.
    disallowedTools: ['shell', 'edit', 'delete'],
    mcpServers,
    local: {
      cwd: opts.cwd,
      settingSources: ['project'],
    },
  });
}

export async function createIntentAgent(
  opts: Omit<AgentFactoryOptions, 'tools' | 'enableCodegraphMcp'>,
): Promise<SDKAgent> {
  return Agent.create({
    apiKey: opts.apiKey,
    model: { id: opts.model },
    name: opts.name,
    systemPrompt: opts.systemPrompt,
    tools: [],
    local: {
      cwd: opts.cwd,
      settingSources: ['project'],
    },
  });
}

export async function runAgentPrompt(
  agent: SDKAgent,
  prompt: string,
): Promise<{ text: string; requestId?: string }> {
  const run = await agent.send(prompt);
  let text = '';
  for await (const event of run.stream()) {
    logStreamEvent(event);
    if (event.type === 'assistant') {
      for (const block of event.message.content) {
        if (block.type === 'text') text += block.text;
      }
    }
  }
  const result = await run.wait();
  if (result.status === 'error') {
    throw new Error(
      `agent run failed: ${result.error?.message ?? result.status}`,
    );
  }
  if (!text.trim() && typeof result.result === 'string') {
    text = result.result;
  }
  return { text, requestId: result.requestId ?? result.id };
}

function logStreamEvent(event: unknown): void {
  if (!event || typeof event !== 'object') return;
  const e = event as { type?: string; message?: unknown; toolCall?: unknown };
  if (e.type === 'tool_call' || e.type === 'tool_use') {
    console.log(`[tool] ${JSON.stringify(e).slice(0, 500)}`);
  } else if (e.type === 'assistant') {
    // already accumulating; brief log
  } else if (e.type === 'status') {
    console.log(`[status] ${JSON.stringify(e).slice(0, 300)}`);
  }
}

export async function disposeAgent(agent: SDKAgent | null | undefined): Promise<void> {
  if (!agent) return;
  try {
    agent.close();
  } catch (err) {
    console.warn(
      `agent dispose failed: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}
