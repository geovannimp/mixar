import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';

/**
 * Positive integer env override. An override that is present but not a positive
 * integer is a misconfiguration, so it throws instead of quietly falling back to the
 * default and hiding the typo.
 */
function envInt(name: string, fallback: number): number {
  const raw = process.env[name]?.trim();
  if (!raw) return fallback;
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0) {
    throw new Error(`pi-review extension: ${name} must be a positive integer, got "${raw}"`);
  }
  return n;
}

export function registerProviders(pi: ExtensionAPI): void {
  // The reviewer model is not in Pi's built-in catalog, so register the endpoint as
  // a provider. This must happen during extension load (extensions load before the
  // action resolves the model), and the workflow must not pass base_url.
  // Trimmed: a whitespace-only value is truthy and would register an empty id/URL.
  const model = process.env.PI_REVIEW_MODEL?.trim();
  // The SDK appends /chat/completions to baseUrl, so accept either the bare base
  // URL or the full endpoint, and normalise a trailing slash so we never build
  // ".../v1//chat/completions".
  const baseUrl = process.env.PI_REVIEW_URL?.trim()
    .replace(/\/chat\/completions\/?$/, '')
    .replace(/\/+$/, '');
  if (!model || !baseUrl) {
    throw new Error('pi-review extension: PI_REVIEW_MODEL and PI_REVIEW_URL must be set');
  }
  // One source for the id, set once in the workflow and passed to the action as
  // `provider:`. A mismatch otherwise fails with a bare "Model not found".
  const providerId = process.env.PI_REVIEW_PROVIDER?.trim() || 'pi-review';
  pi.registerProvider(providerId, {
    baseUrl,
    api: 'openai-completions',
    // The SDK expands `$NAME` from the process environment, so the workflow has to
    // export the secret under exactly this name.
    apiKey: '$PI_REVIEW_AUTH_TOKEN',
    models: [
      {
        id: model,
        name: `Pi review model (${model})`,
        reasoning: false,
        input: ['text'],
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        // Override when the endpoint's model differs from these defaults.
        contextWindow: envInt('PI_REVIEW_CONTEXT_WINDOW', 128_000),
        maxTokens: envInt('PI_REVIEW_MAX_TOKENS', 4096),
      },
    ],
  });
}
