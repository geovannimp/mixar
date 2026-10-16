/**
 * Marker required on the root comment of a thread this action may resolve.
 * The workflow prompt instructs the model to prefix every inline finding body
 * with it, because the review itself is posted by the action's built-in
 * `create_pull_request_review` tool rather than by this extension.
 */
export const FINDING_MARKER = '<!-- pi-code-review -->';
