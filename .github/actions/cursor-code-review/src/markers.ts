export const FINDING_MARKER = '<!-- cursor-code-review -->';
export const SUMMARY_ANCHOR = '<!-- cursor-code-review-summary -->';

export function bodyHasOurMarker(body: string | null | undefined): boolean {
  if (!body) return false;
  return body.includes(FINDING_MARKER) || body.includes(SUMMARY_ANCHOR);
}

export function prefixFindingBody(body: string): string {
  if (body.includes(FINDING_MARKER)) return body;
  return `${FINDING_MARKER}\n${body}`;
}

export function formatFindingComment(opts: {
  severity: string;
  category: string;
  title: string;
  body: string;
}): string {
  const header = `**[${opts.severity}/${opts.category}] ${opts.title}**`;
  return prefixFindingBody(`${header}\n\n${opts.body.trim()}`);
}

export function formatSummaryBody(markdown: string): string {
  const trimmed = markdown.trim();
  if (trimmed.includes(SUMMARY_ANCHOR)) return trimmed;
  return `${SUMMARY_ANCHOR}\n${trimmed}`;
}
