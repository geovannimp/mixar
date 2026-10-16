import type { ExtensionAPI } from '@earendil-works/pi-coding-agent';
import { loadRuntime } from './runtime.ts';
import { replyToReviewComment, resolveReviewThread } from './threads.ts';

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

/**
 * The only two tools this extension provides: the action's built-in GitHub tools
 * cover the review itself (create_pull_request_review, get_pr_diff), but nothing
 * there replies to a review comment or resolves a review thread.
 */
export function registerTools(pi: ExtensionAPI): void {
  const register = (tool: ToolSpec) =>
    pi.registerTool(tool as unknown as Parameters<ExtensionAPI['registerTool']>[0]);

  register({
    name: 'reply_to_review_comment',
    label: 'Reply to review comment',
    description:
      'Reply in an inline PR review thread. The reply is tagged with the bot marker.',
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
    description:
      'Resolve the inline review thread containing the given PR review comment. Only threads whose root comment carries the pi-review marker are resolved.',
    parameters: commentIdSchema,
    execute: async (_id, params) =>
      textResult(await resolveReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
  });
}
