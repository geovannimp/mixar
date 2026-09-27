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

/** A path boundary, so that `mixar-fork` is not counted as `mixar`. */
const REPO_PATH_BOUNDARY = /[/?#]/;

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
