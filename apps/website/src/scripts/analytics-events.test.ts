import { describe, expect, it } from "vitest";

import {
  CONTRIBUTING_URL,
  GITHUB_REPO,
  QUICK_START_URL,
  README_URL,
  TECH_SPEC_URL,
} from "../consts";
import { resolveDestination } from "./analytics-events";

describe("resolveDestination", () => {
  // Every outbound CTA href comes from src/consts.ts, so each one must map to
  // its own bucket. GITHUB_REPO is a prefix of all the others, which is the
  // trap: a first-match-wins scan in declaration order sends quick_start,
  // readme, tech_spec and contributing all to `repo`, and the funnel silently
  // reports one undifferentiated bucket.
  it("maps the tech spec link to tech_spec", () => {
    expect(resolveDestination(TECH_SPEC_URL)).toBe("tech_spec");
  });

  it("maps the quick start link to quick_start", () => {
    expect(resolveDestination(QUICK_START_URL)).toBe("quick_start");
  });

  it("maps the readme link to readme", () => {
    expect(resolveDestination(README_URL)).toBe("readme");
  });

  it("maps the contributing link to contributing", () => {
    expect(resolveDestination(CONTRIBUTING_URL)).toBe("contributing");
  });

  it("maps the bare repository link to repo", () => {
    expect(resolveDestination(GITHUB_REPO)).toBe("repo");
  });

  // Footer.astro:96 links to the GPL text. Counting it as a repo click would
  // inflate the funnel's conversion rate with a licensing link.
  it("returns null for a non-repository external link", () => {
    expect(resolveDestination("https://www.gnu.org/licenses/gpl-3.0.html")).toBeNull();
  });

  it("returns null for another GitHub repository", () => {
    expect(resolveDestination("https://github.com/someone/else")).toBeNull();
  });

  it("returns null for an internal link", () => {
    expect(resolveDestination("/developers")).toBeNull();
  });

  it("returns null for an empty href", () => {
    expect(resolveDestination("")).toBeNull();
  });

  // A future /issues or /releases link is still "the repo".
  it("falls back to repo for an unlisted path inside the repository", () => {
    expect(resolveDestination(`${GITHUB_REPO}/issues`)).toBe("repo");
  });

  it("does not treat a similarly named repository as ours", () => {
    expect(resolveDestination("https://github.com/geovannimp/mixar-fork")).toBeNull();
  });

  it("tolerates surrounding whitespace", () => {
    expect(resolveDestination(`  ${GITHUB_REPO}  `)).toBe("repo");
  });
});
