import {
  CATEGORIES,
  CONFIDENCE_FLOOR,
  FINDING_MARKER,
  SEVERITIES,
  SUMMARY_ANCHOR,
} from './constants.ts';
import { clampToAnchor, isContiguous } from './diff.ts';
import type {
  Category,
  DiffAnchors,
  ReviewCommentInput,
  ReviewFinding,
  Severity,
} from './types.ts';

export function splitByConfidence(
  findings: ReviewFinding[],
  floor = CONFIDENCE_FLOOR,
): { kept: ReviewFinding[]; dropped: number } {
  const kept = findings.filter((f) => f.confidence >= floor);
  return { kept, dropped: findings.length - kept.length };
}

export function formatFindingBody(
  f: Pick<ReviewFinding, 'severity' | 'category' | 'title' | 'body'>,
): string {
  return `${FINDING_MARKER}\n**[${f.severity}/${f.category}] ${f.title}**\n\n${f.body.trim()}`;
}

/**
 * Turn findings into inline comments anchored to the diff. Findings whose end line
 * cannot be clamped onto the diff are returned as `outside`.
 */
export function buildReviewComments(
  findings: ReviewFinding[],
  anchors: DiffAnchors,
): { comments: ReviewCommentInput[]; outside: ReviewFinding[] } {
  const comments: ReviewCommentInput[] = [];
  const outside: ReviewFinding[] = [];
  for (const f of findings) {
    const endAnchor = clampToAnchor(anchors, f.path, f.end_line ?? f.start_line);
    if (endAnchor === null) {
      outside.push(f);
      continue;
    }
    // Span both ends onto anchors, then order them: the two nearest anchors can
    // cross when the end snaps down and the start snaps up. Emitting max/min keeps
    // the range valid and preserves the reviewer's span instead of dropping it.
    let startAnchor = endAnchor;
    if (f.end_line != null && f.end_line !== f.start_line) {
      startAnchor = clampToAnchor(anchors, f.path, f.start_line) ?? endAnchor;
    }
    const line = Math.max(startAnchor, endAnchor);
    const start = Math.min(startAnchor, endAnchor);
    const comment: ReviewCommentInput = {
      path: f.path,
      line,
      side: 'RIGHT',
      body: formatFindingBody(f),
    };
    // Multi-line comments are only valid when the whole range is in the diff;
    // GitHub rejects a range spanning hunks with 422. Collapse to one line when
    // the two anchors are not contiguous.
    const fileAnchors = anchors.get(f.path);
    if (start < line && fileAnchors && isContiguous(fileAnchors, start, line)) {
      comment.start_line = start;
      comment.start_side = 'RIGHT';
    }
    comments.push(comment);
  }
  return { comments, outside };
}

export function buildSummaryBody(markdown: string, outside: ReviewFinding[]): string {
  let body = markdown.trim();
  if (!body.includes(SUMMARY_ANCHOR)) body = `${SUMMARY_ANCHOR}\n${body}`;
  if (outside.length > 0) {
    const bullets = outside
      .map(
        (f) =>
          `- \`${f.path}:${f.end_line ?? f.start_line}\` [${f.severity}/${f.category}] ${f.title}`,
      )
      .join('\n');
    body += `\n\n### Findings outside the diff\n\n${bullets}`;
  }
  return body;
}

/**
 * Validate the findings array before it reaches the comment builder, so a
 * malformed entry raises a clear tool error instead of an opaque runtime one.
 */
export function parseFindings(value: unknown): ReviewFinding[] {
  if (!Array.isArray(value)) throw new Error('findings must be an array');
  return value.map((raw, i) => {
    const where = `findings[${i}]`;
    const f = (raw ?? {}) as Record<string, unknown>;
    if (typeof f.path !== 'string' || f.path === '') {
      throw new Error(`${where}.path must be a non-empty string`);
    }
    if (!Number.isInteger(f.start_line) || (f.start_line as number) < 1) {
      throw new Error(`${where}.start_line must be a positive integer`);
    }
    if (
      f.end_line != null &&
      (!Number.isInteger(f.end_line) || (f.end_line as number) < 1)
    ) {
      throw new Error(`${where}.end_line must be a positive integer when set`);
    }
    // Reject an inverted range here rather than letting buildReviewComments
    // silently reorder it: this function is the validation gate for the tool.
    if (f.end_line != null && (f.end_line as number) < (f.start_line as number)) {
      throw new Error(`${where}.end_line (${f.end_line}) must be >= start_line (${f.start_line})`);
    }
    if (!SEVERITIES.includes(f.severity as Severity)) {
      throw new Error(`${where}.severity must be one of: ${SEVERITIES.join(', ')}`);
    }
    if (!CATEGORIES.includes(f.category as Category)) {
      throw new Error(`${where}.category must be one of: ${CATEGORIES.join(', ')}`);
    }
    if (typeof f.title !== 'string' || f.title === '') {
      throw new Error(`${where}.title must be a non-empty string`);
    }
    if (typeof f.body !== 'string' || f.body === '') {
      throw new Error(`${where}.body must be a non-empty string`);
    }
    // Number.isFinite is required: NaN is a number that compares false to both
    // bounds, so it would pass and then be silently dropped by splitByConfidence.
    if (typeof f.confidence !== 'number' || !Number.isFinite(f.confidence) || f.confidence < 0 || f.confidence > 1) {
      throw new Error(`${where}.confidence must be a number in [0, 1]`);
    }
    return {
      path: f.path,
      start_line: f.start_line as number,
      // null passes the `!= null` check above, so normalize it: the field is
      // typed number | undefined and a stored null would be a type lie.
      end_line: f.end_line == null ? undefined : (f.end_line as number),
      severity: f.severity as Severity,
      category: f.category as Category,
      title: f.title,
      body: f.body,
      confidence: f.confidence,
    };
  });
}
