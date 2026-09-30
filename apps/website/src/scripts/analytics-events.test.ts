import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  CONTRIBUTING_URL,
  GITHUB_REPO,
  QUICK_START_URL,
  README_URL,
  TECH_SPEC_URL,
} from "../consts";
import { PLACEMENTS, resolveDestination } from "./analytics-events";

const COMPONENTS_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "components");

function astroFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return astroFiles(path);
    return entry.name.endsWith(".astro") ? [path] : [];
  });
}

/** Every `placement="x"` / `data-placement="x"` value across the components. */
function placementsInUse(): string[] {
  const attribute = /(?:data-)?placement="([^"]+)"/g;
  return astroFiles(COMPONENTS_DIR).flatMap((file) => {
    const source = readFileSync(file, "utf8");
    return [...source.matchAll(attribute)].map((match) => match[1] as string);
  });
}

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

  // The bug the single-segment test above cannot catch: the boundary check ran
  // over the whole remainder, so any path *containing* `/` matched. A fork with
  // a subpath counted as our repo and inflated the funnel.
  it("does not treat a subpath of a similarly named repository as ours", () => {
    expect(resolveDestination("https://github.com/geovannimp/mixar-fork/issues")).toBeNull();
  });

  it("does not treat a query on a similarly named repository as ours", () => {
    expect(resolveDestination("https://github.com/geovannimp/mixar-fork?tab=readme")).toBeNull();
  });

  it("tolerates surrounding whitespace", () => {
    expect(resolveDestination(`  ${GITHUB_REPO}  `)).toBe("repo");
  });
});

describe("PLACEMENTS", () => {
  // A misspelled placement is non-empty, so the `unknown` fallback does not
  // catch it: `dev-hero` would silently fork the `dev_hero` bucket in two.
  // `tsc` cannot help here because it does not parse .astro files, so the
  // closed set is enforced against the component sources instead.
  it("accepts every placement used by a component", () => {
    expect(placementsInUse().filter((value) => !PLACEMENTS.includes(value as never))).toEqual(
      [],
    );
  });

  it("is actually used, so the set cannot rot into fiction", () => {
    expect(placementsInUse().length).toBeGreaterThan(0);
  });

  it("has no duplicates", () => {
    expect(new Set(PLACEMENTS).size).toBe(PLACEMENTS.length);
  });
});
