import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { registerMcpTeardown } from './src/mcp-teardown.ts';
import { registerTools } from './src/tools.ts';

export default function (pi: ExtensionAPI): void {
  registerTools(pi);
  registerMcpTeardown(pi);
}
