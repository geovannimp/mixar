import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { registerProviders } from './src/providers.ts';
import { registerTools } from './src/tools.ts';

export default function (pi: ExtensionAPI): void {
  registerProviders(pi);
  registerTools(pi);
}
