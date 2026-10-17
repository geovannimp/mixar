import type { ExtensionAPI, ToolDefinition } from '@earendil-works/pi-coding-agent';
import { loadRuntime } from './runtime.ts';
import { replyToReviewComment, resolveReviewThread } from './threads.ts';

const textResult = (text: string) => ({
  content: [{ type: 'text' as const, text }],
  details: {},
});

function intParam(params: unknown, key: string): number {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 1) {
    throw new Error(`${key} must be a positive integer`);
  }
  return value;
}

/** Unlike a bare typeof check, this rejects an empty or whitespace-only body. */
function bodyParam(params: unknown, key: string): string {
  const value = (params as Record<string, unknown> | null | undefined)?.[key];
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`${key} must be a non-empty string`);
  }
  return value;
}

/** The comment_id property, shared by both tools so the definition cannot drift. */
const commentIdProperty = {
  type: 'integer',
  minimum: 1,
  description: 'PR review comment id',
} as const;

const commentIdSchema = {
  type: 'object',
  properties: { comment_id: commentIdProperty },
  required: ['comment_id'],
} as const;

/**
 * The only two tools this extension provides: the action's built-in GitHub tools
 * cover the review itself (create_pull_request_review, get_pr_diff), but nothing
 * there replies to a review comment or resolves a review thread.
 */
export function registerTools(pi: ExtensionAPI): void {
  const tools: ToolDefinition[] = [
    {
      name: 'reply_to_review_comment',
      label: 'Reply to review comment',
      description:
        'Reply in an inline PR review thread. The reply is tagged with the bot marker.',
      parameters: {
        type: 'object',
        properties: {
          comment_id: commentIdProperty,
          body: { type: 'string' },
        },
        required: ['comment_id', 'body'],
      },
      execute: async (_toolCallId, params) =>
        textResult(
          await replyToReviewComment(
            loadRuntime(),
            intParam(params, 'comment_id'),
            bodyParam(params, 'body'),
          ),
        ),
    },
    {
      name: 'resolve_review_thread',
      label: 'Resolve review thread',
      description:
        'Resolve the inline review thread containing the given PR review comment. Only threads opened by this action on this PR can be resolved.',
      parameters: commentIdSchema,
      execute: async (_toolCallId, params) =>
        textResult(await resolveReviewThread(loadRuntime(), intParam(params, 'comment_id'))),
    },
  ];

  for (const tool of tools) pi.registerTool(tool);
}
