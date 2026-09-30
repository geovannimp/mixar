import {
  CONTRIBUTING_URL,
  GITHUB_REPO,
  QUICK_START_URL,
  README_URL,
  TECH_SPEC_URL,
} from "../consts";

/**
 * Closed set of outbound destinations we count.
 *
 * A closed enum rather than a free-text href is what lets the download funnel
 * be built without regex, and it keeps query strings out of PostHog entirely.
 */
export const DESTINATIONS = {
  repo: "repo",
  quickStart: "quick_start",
  readme: "readme",
  techSpec: "tech_spec",
  contributing: "contributing",
} as const;

export type Destination = (typeof DESTINATIONS)[keyof typeof DESTINATIONS];

/**
 * Closed set of placements a CTA can be annotated with.
 *
 * `placement` is read off the DOM at runtime, so a misspelling is non-empty and
 * slips past the `unknown` fallback — `dev-hero` would quietly split the
 * `dev_hero` bucket in two. The set is enforced against the component sources
 * by `analytics-events.test.ts`; this export is the single definition both the
 * test and `Button.astro` refer to.
 */
export const PLACEMENTS = [
  "header",
  "hero",
  "platforms",
  "developers",
  "dev_hero",
  "dev_architecture",
  "dev_docs",
  "footer",
] as const;

export type Placement = (typeof PLACEMENTS)[number];

/**
 * `GITHUB_REPO` is a prefix of every other consts URL, so the scan is an exact
 * match across all of them first; the bare-repository fallback is considered
 * only afterwards. A first-match-wins scan in declaration order would collapse
 * quick_start, readme, tech_spec and contributing into `repo`.
 */
const EXACT_BY_URL: ReadonlyArray<readonly [string, Destination]> = [
  [TECH_SPEC_URL, DESTINATIONS.techSpec],
  [QUICK_START_URL, DESTINATIONS.quickStart],
  [README_URL, DESTINATIONS.readme],
  [CONTRIBUTING_URL, DESTINATIONS.contributing],
];

/**
 * Anchored: the character immediately after the repository must start a path,
 * query or fragment. An unanchored test scans the whole remainder, which would
 * accept `mixar-fork/issues` — the `/` is present, just not in the right place.
 */
const REPO_PATH_BOUNDARY = /^[/?#]/;

export function resolveDestination(href: string): Destination | null {
  const url = href.trim();
  if (url === "") return null;

  for (const [candidate, destination] of EXACT_BY_URL) {
    if (url === candidate) return destination;
  }

  if (url === GITHUB_REPO) return DESTINATIONS.repo;

  const rest = url.startsWith(GITHUB_REPO) ? url.slice(GITHUB_REPO.length) : "";
  if (rest !== "" && REPO_PATH_BOUNDARY.test(rest)) return DESTINATIONS.repo;

  return null;
}
