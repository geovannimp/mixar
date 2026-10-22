import type { ExtensionAPI, ToolDefinition } from '@earendil-works/pi-coding-agent';

import { executeCodegraph, type CodegraphParams } from './codegraph.ts';
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

const codegraphSchema = {
  type: 'object',
  properties: {
    action: {
      type: 'string',
      enum: ['explore', 'node', 'callers', 'callees', 'impact'],
    },
    query: {
      type: 'string',
      description:
        'For explore: symbol names, file names, or a question. For the other actions: the symbol to look up.',
    },
    file: {
      type: 'string',
      description: 'Repo-relative file that narrows the lookup to one file',
    },
    limit: {
      type: 'number',
      description: 'Maximum results per definition (callers, callees)',
    },
    depth: { type: 'number', description: 'Traversal depth (impact)' },
    maxFiles: {
      type: 'number',
      description: 'Maximum files to include source from (explore)',
    },
  },
  required: ['action', 'query'],
  additionalProperties: false,
} as const;

/**
 * The extension's tools: a narrow wrapper around the deterministic OCR
 * delegate commands, and a wrapper around the CodeGraph CLI for code
 * intelligence. Neither runs an LLM.
 */
export function registerTools(pi: ExtensionAPI): void {
  const ocr: ToolDefinition = {
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

  const codegraph: ToolDefinition = {
    name: 'codegraph',
    label: 'CodeGraph code intelligence',
    description:
      'Query the CodeGraph index of the code under review. action=explore is the primary one: it returns the verbatim source of the relevant symbols plus their call paths and blast radius. action=node returns one symbol with its callers and callees; action=callers, action=callees and action=impact answer those questions for one symbol. Never runs an LLM.',
    parameters: codegraphSchema,
    execute: async (_toolCallId, params) => {
      const { output } = await executeCodegraph(params as CodegraphParams);
      return textResult(output);
    },
  };

  pi.registerTool(ocr);
  pi.registerTool(codegraph);
}
