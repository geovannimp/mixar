import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { CATEGORIES, SEVERITIES } from './constants.ts';
import { listReviewFiles } from './files.ts';
import { parseFindings } from './findings.ts';
import { postReview } from './reviews.ts';
import { loadRuntime } from './runtime.ts';
import { getReviewThread, replyToReviewComment, resolveReviewThread } from './threads.ts';

interface ToolSpec {
  name: string;
  label: string;
  description: string;
  parameters: Record<string, unknown>;
  execute: (toolCallId: string, params: unknown) => Promise<ToolResult>;
}

interface ToolResult {
  content: Array<{ type: 'text'; text: string }>;
  details: Record<string, never>;
}

const textResult = (text: string): ToolResult => ({
  content: [{ type: 'text', text }],
  details: {},
});

function intParam(params: unknown, key: string): number {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 1) {
    throw new Error(`${key} must be a positive integer`);
  }
  return value;
}

function stringParam(params: unknown, key: string): string {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'string') throw new Error(`${key} must be a string`);
  return value;
}

const commentIdSchema = {
  type: 'object',
  properties: { comment_id: { type: 'integer', minimum: 1 } },
  required: ['comment_id'],
};

export function registerTools(pi: ExtensionAPI): void {
  const register = (tool: ToolSpec) =>
    pi.registerTool(tool as unknown as Parameters<ExtensionAPI['registerTool']>[0]);

  register({
    name: 'list_review_files',
    label: 'List review files',
    description:
      'List changed PR files to review (removed and excluded paths dropped) with base-branch path rules that apply to them.',
    parameters: { type: 'object', properties: {}, required: [] },
    execute: async () => textResult(await listReviewFiles(loadRuntime())),
  });

  register({
    name: 'get_review_thread',
    label: 'Get review thread',
    description:
      'Fetch the full inline review thread containing a PR review comment: root, replies (oldest first), and the comment node id.',
    parameters: commentIdSchema,
    execute: async (_id, params) =>
      textResult(await getReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
  });

  register({
    name: 'reply_to_review_comment',
    label: 'Reply to review comment',
    description: 'Reply in an inline PR review thread. The reply is tagged with the bot marker.',
    parameters: {
      type: 'object',
      properties: {
        comment_id: { type: 'integer', minimum: 1 },
        body: { type: 'string' },
      },
      required: ['comment_id', 'body'],
    },
    execute: async (_id, params) =>
      textResult(
        await replyToReviewComment(
          loadRuntime(),
          intParam(params, 'comment_id'),
          stringParam(params, 'body'),
        ),
      ),
  });

  register({
    name: 'resolve_review_thread',
    label: 'Resolve review thread',
    description: 'Resolve the inline review thread containing the given PR review comment.',
    parameters: commentIdSchema,
    execute: async (_id, params) =>
      textResult(await resolveReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
  });

  register({
    name: 'post_review',
    label: 'Post review',
    description:
      'Post the review: drops low-confidence findings, anchors the rest to diff lines, upserts the sticky summary, and posts inline comments in batches.',
    parameters: {
      type: 'object',
      properties: {
        summary_markdown: { type: 'string' },
        findings: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              path: { type: 'string' },
              start_line: { type: 'integer', minimum: 1 },
              end_line: { type: 'integer', minimum: 1 },
              severity: { type: 'string', enum: [...SEVERITIES] },
              category: { type: 'string', enum: [...CATEGORIES] },
              title: { type: 'string' },
              body: { type: 'string' },
              confidence: { type: 'number', minimum: 0, maximum: 1 },
            },
            required: [
              'path',
              'start_line',
              'severity',
              'category',
              'title',
              'body',
              'confidence',
            ],
          },
        },
      },
      required: ['summary_markdown', 'findings'],
    },
    execute: async (_id, params) => {
      const p = (params ?? {}) as { findings?: unknown };
      return textResult(
        await postReview(loadRuntime(), {
          summary_markdown: stringParam(p, 'summary_markdown'),
          findings: parseFindings(p.findings),
        }),
      );
    },
  });
}
