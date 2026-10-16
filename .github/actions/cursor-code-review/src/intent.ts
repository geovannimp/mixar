import { z } from 'zod';
import { extractLastJsonFence } from './findings.js';

export const intentSchema = z.enum([
  'question',
  'dispute',
  'fixed',
  'ack',
  're_review',
  'add_value',
  'unrelated',
]);

export const intentResultSchema = z.object({
  intent: intentSchema,
  should_reply: z.boolean(),
  rationale: z.string(),
});

export type IntentResult = z.infer<typeof intentResultSchema>;

export const disputeResultSchema = z.object({
  verdict: z.enum(['stand_by', 'withdraw']),
  reply_markdown: z.string(),
});

export const fixedResultSchema = z.object({
  verified: z.boolean(),
  reply_markdown: z.string(),
});

export type DisputeResult = z.infer<typeof disputeResultSchema>;
export type FixedResult = z.infer<typeof fixedResultSchema>;

/** Strip HTML comments / invisible chars (Kodus hardening). */
export function sanitizeCommentBody(body: string): string {
  let out = withoutHiddenMarkup(body);
  // Zero-width / bidi controls
  out = out.replace(/[\u200B-\u200F\u202A-\u202E\u2060\uFEFF]/g, '');
  return out.trim();
}

function withoutHiddenMarkup(body: string): string {
  let prev = '';
  let cur = body;
  while (cur !== prev) {
    prev = cur;
    cur = stripOnePass(cur);
  }
  return cur;
}

function stripOnePass(body: string): string {
  let out = '';
  let rest = body;
  let start = nextHiddenMarkup(rest);
  while (start >= 0) {
    out += rest.slice(0, start);
    if (rest.startsWith('<!--', start)) {
      const close = rest.indexOf('-->', start + 4);
      rest = close < 0 ? '' : rest.slice(close + 3);
    } else {
      const close = rest.indexOf('>', start + 2);
      rest = close < 0 ? '' : rest.slice(close + 1);
    }
    start = nextHiddenMarkup(rest);
  }
  return out + rest;
}

function nextHiddenMarkup(s: string): number {
  const a = s.indexOf('<!--');
  const b = s.indexOf('<!');
  const c = s.indexOf('<?');
  const candidates = [a, b, c].filter((i) => i >= 0);
  return candidates.length === 0 ? -1 : Math.min(...candidates);
}

export function parseIntentResult(
  text: string,
  fallback: { mentionsUs: boolean },
): IntentResult {
  try {
    const raw = extractLastJsonFence(text) ?? text.trim();
    const parsed = JSON.parse(raw) as unknown;
    return intentResultSchema.parse(parsed);
  } catch {
    if (fallback.mentionsUs) {
      return {
        intent: 'question',
        should_reply: true,
        rationale: 'parse failure; newest message addresses us',
      };
    }
    return {
      intent: 'unrelated',
      should_reply: false,
      rationale: 'parse failure; no address signal',
    };
  }
}

export function mentionsCursorReview(body: string): boolean {
  const lower = body.toLowerCase();
  return (
    lower.includes('@cursor-review') ||
    lower.includes('/cursor-review') ||
    lower.includes('cursor-code-review') ||
    lower.includes('cursor review')
  );
}
