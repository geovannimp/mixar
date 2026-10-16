import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { registerTools } from './src/tools.ts';

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
  pi.registerProvider('ocr-llm', {
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
        contextWindow: 128000,
        maxTokens: 4096,
      },
    ],
  });

  registerTools(pi);
}
