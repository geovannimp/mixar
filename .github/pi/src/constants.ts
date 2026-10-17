/**
 * Marker this extension tags its replies with, so a human can tell a bot reply from a
 * reviewer's. It deliberately does NOT authorise anything: it is a public literal that
 * any commenter can copy, so `resolve_review_thread` checks authorship instead.
 */
export const FINDING_MARKER = '<!-- pi-code-review -->';
