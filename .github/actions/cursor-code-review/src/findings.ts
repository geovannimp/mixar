import { z } from 'zod';

export const severitySchema = z.enum([
  'critical',
  'high',
  'medium',
  'low',
]);

export const categorySchema = z.enum([
  'bug',
  'security',
  'performance',
  'maintainability',
  'test',
  'style',
  'documentation',
  'other',
]);

export const reviewFindingSchema = z.object({
  path: z.string().min(1),
  start_line: z.number().int().positive(),
  end_line: z.number().int().positive().optional(),
  severity: severitySchema,
  category: categorySchema,
  title: z.string().min(1),
  body: z.string().min(1),
  confidence: z.number().min(0).max(1),
});

export const reviewResultSchema = z.object({
  summary_markdown: z.string(),
  findings: z.array(reviewFindingSchema),
});

export type ReviewFinding = z.infer<typeof reviewFindingSchema>;
export type ReviewResult = z.infer<typeof reviewResultSchema>;

export const CONFIDENCE_FLOOR = 0.55;

/** Extract the last fenced JSON object from agent text. */
export function extractLastJsonFence(text: string): string | null {
  const fenceRe = /```(?:json)?\s*([\s\S]*?)```/gi;
  let last: string | null = null;
  let match: RegExpExecArray | null;
  while ((match = fenceRe.exec(text)) !== null) {
    const candidate = match[1]?.trim();
    if (candidate && (candidate.startsWith('{') || candidate.startsWith('['))) {
      last = candidate;
    }
  }
  if (last) return last;

  // Fallback: last top-level object-looking span
  const start = text.lastIndexOf('{');
  if (start < 0) return null;
  const slice = text.slice(start);
  try {
    JSON.parse(slice);
    return slice;
  } catch {
    // try balanced brace walk
    let depth = 0;
    for (let i = 0; i < slice.length; i++) {
      const ch = slice[i];
      if (ch === '{') depth++;
      else if (ch === '}') {
        depth--;
        if (depth === 0) {
          return slice.slice(0, i + 1);
        }
      }
    }
  }
  return null;
}

export function parseReviewResult(text: string): ReviewResult {
  const raw = extractLastJsonFence(text);
  if (!raw) {
    throw new Error('No JSON fence found in agent output');
  }
  const parsed = JSON.parse(raw) as unknown;
  return reviewResultSchema.parse(parsed);
}

export function filterFindings(
  result: ReviewResult,
  changedPaths: Set<string>,
  confidenceFloor = CONFIDENCE_FLOOR,
): ReviewResult {
  const findings = result.findings.filter((f) => {
    if (f.confidence < confidenceFloor) return false;
    if (!changedPaths.has(f.path)) return false;
    return true;
  });
  return { summary_markdown: result.summary_markdown, findings };
}

export function mergeReviewResults(parts: ReviewResult[]): ReviewResult {
  const findings: ReviewFinding[] = [];
  const summaries: string[] = [];
  for (const part of parts) {
    findings.push(...part.findings);
    if (part.summary_markdown.trim()) summaries.push(part.summary_markdown.trim());
  }
  return {
    summary_markdown: summaries.join('\n\n'),
    findings,
  };
}
