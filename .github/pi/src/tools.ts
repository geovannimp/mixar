import type { ExtensionAPI, ToolDefinition } from '@earendil-works/pi-coding-agent';
import { executeOcr, type OcrParams } from './ocr.ts';

const textResult = (text: string) => ({
  content: [{ type: 'text' as const, text }],
  details: {},
});

const ocrSchema = {
  type: 'object',
  properties: {
    action: { type: 'string', enum: ['preview', 'rule'] },
    from: { type: 'string', description: 'Base ref for preview range mode' },
    to: { type: 'string', description: 'Head ref for preview range mode' },
    commit: { type: 'string', description: 'Single commit SHA for preview commit mode' },
    paths: {
      type: 'array',
      items: { type: 'string' },
      description: 'Repo-relative paths for rule mode',
    },
    repo: { type: 'string', description: 'Repo inside the workspace; defaults to pr-head or the workspace' },
    rule: { type: 'string', description: 'Rule file inside the workspace; defaults to .opencodereview/rule.json' },
  },
  required: ['action'],
  additionalProperties: false,
} as const;

/**
 * The only extension tool: a narrow wrapper around the deterministic OCR
 * delegate commands. It never runs the LLM-backed review.
 */
export function registerTools(pi: ExtensionAPI): void {
  const tool: ToolDefinition = {
    name: 'ocr',
    label: 'OCR review plan',
    description:
      'Run the pinned OpenCodeReview CLI in delegate mode. action=preview lists reviewable files and exclusions; action=rule returns the review checklist grouped by rule. Never runs an LLM.',
    parameters: ocrSchema,
    execute: async (_toolCallId, params) => {
      const { raw, ...value } = await executeOcr(params as OcrParams);
      void raw;
      return textResult(JSON.stringify(value, null, 1));
    },
  };
  pi.registerTool(tool);
}
