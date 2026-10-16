export const FINDING_MARKER = '<!-- pi-code-review -->';
export const SUMMARY_ANCHOR = '<!-- pi-code-review-summary -->';
export const CONFIDENCE_FLOOR = 0.55;
export const DEFAULT_BATCH_SIZE = 50;
export const REVIEW_CONFIG_PATH = '.pi-review/config.json';
export const SEVERITIES = ['critical', 'high', 'medium', 'low'] as const;
export const CATEGORIES = [
  'bug',
  'security',
  'performance',
  'maintainability',
  'test',
  'style',
  'documentation',
  'other',
] as const;

export const REVIEW_BODY_DONE = `${FINDING_MARKER}\nReview posted; see summary comment.`;
