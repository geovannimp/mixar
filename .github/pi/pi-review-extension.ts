import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { registerTools } from './src/tools.ts';

/** Positive integer env override, else the default. */
function envInt(name: string, fallback: number): number {
  const n = Number(process.env[name]);
  return Number.isInteger(n) && n > 0 ? n : fallback;
}

export default function (pi: ExtensionAPI): void {
  // The reviewer model is not in Pi's built-in catalog, so register the OCR
  // endpoint as a provider. This must happen here (extensions load before the
  // action resolves the model), and the workflow must not pass base_url.
  const ocrModel = process.env.OCR_LLM_MODEL;
  // The SDK appends /chat/completions to baseUrl, so accept the full endpoint too.
  const ocrBaseUrl = process.env.OCR_LLM_URL?.replace(/\/chat\/completions\/?$/, '');
  if (!ocrModel || !ocrBaseUrl) {
    throw new Error('pi-review extension: OCR_LLM_MODEL and OCR_LLM_URL must be set');
  }
  // One source for the id, set once in the workflow and passed to the action as
  // `provider:`. A mismatch otherwise fails with a bare "Model not found".
  const providerId = process.env.PI_REVIEW_PROVIDER?.trim() || 'ocr-llm';
  pi.registerProvider(providerId, {
    baseUrl: ocrBaseUrl,
    api: 'openai-completions',
    apiKey: '$OCR_LLM_AUTH_TOKEN',
    models: [
      {
        id: ocrModel,
        name: `OCR review model (${ocrModel})`,
        reasoning: false,
        input: ['text'],
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        // Override when the endpoint's model differs from these defaults.
        contextWindow: envInt('PI_REVIEW_CONTEXT_WINDOW', 128_000),
        maxTokens: envInt('PI_REVIEW_MAX_TOKENS', 4096),
      },
    ],
  });

  registerTools(pi);
}
