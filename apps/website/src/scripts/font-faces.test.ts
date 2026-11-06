import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const STYLES = join(HERE, "..", "styles");
// `apps/website/node_modules` and the workspace root both hold the packages;
// resolving through the repo root covers either layout.
const FONTSOURCE = join(HERE, "..", "..", "..", "..", "node_modules", "@fontsource-variable");

const PACKAGES = ["outfit", "space-grotesk", "noto-sans-mono"] as const;

type Face = {
 family: string;
 weight: string;
 /** Subset file name, e.g. `outfit-latin-wght-normal.woff2`. */
 file: string;
 unicodeRange: string;
 display: string | undefined;
};

function parseFaces(css: string): Face[] {
 const faces: Face[] = [];
 for (const [, body] of css.matchAll(/@font-face\s*\{([^}]*)\}/g)) {
  const family = /font-family:\s*['"]([^'"]+)['"]/.exec(body)?.[1];
  const weight = /font-weight:\s*([^;]+);/.exec(body)?.[1]?.trim();
  const file = /url\(\s*["']?[^"')]*\/([^/"']+\.woff2)["']?\s*\)/.exec(body)?.[1];
  const unicodeRange = /unicode-range:\s*([^;]+);/.exec(body)?.[1]?.replace(/\s+/g, "");
  const display = /font-display:\s*(\w+)/.exec(body)?.[1];
  if (!family || !weight || !file || !unicodeRange) {
   throw new Error(`unparsed @font-face: ${body}`);
  }
  faces.push({ family, weight, file, unicodeRange, display });
 }
 return faces;
}

const local = parseFaces(readFileSync(join(STYLES, "fonts.css"), "utf8"));

/**
 * `src/styles/fonts.css` exists only because fontsource hardcodes
 * `font-display: swap`, which shifts the hero on a late face load (0.061 CLS on
 * desktop, the last metric under 100). The file restates the packages' faces so
 * it can say `optional`; if an upgrade adds, drops, or renames a face, the copy
 * silently stops covering the site's glyphs, so this test pins the copy to the
 * installed packages.
 */
describe("self-hosted font faces", () => {
 it("never swaps after first paint", () => {
  expect(local.map((face) => face.display)).toEqual(local.map(() => "optional"));
 });

 it("mirrors the packages' faces one for one", () => {
  const key = ({ family, weight, file, unicodeRange }: Face) =>
   `${family}|${weight}|${file}|${unicodeRange}`;
  const upstream = PACKAGES.flatMap((pkg) =>
   parseFaces(readFileSync(join(FONTSOURCE, pkg, "index.css"), "utf8")),
  );

  // Exact set equality both ways: a face the packages dropped would 404 at
  // runtime, and a face they added would miss the brand font (or lose a
  // script) without a build error.
  expect(local.map(key).sort()).toEqual(upstream.map(key).sort());
  // Upstream ships `swap`; once that changes this file can go away.
  expect(new Set(upstream.map((face) => face.display))).toEqual(new Set(["swap"]));
 });

 it("is the only font source global.css pulls in", () => {
  const global = readFileSync(join(STYLES, "global.css"), "utf8");
  expect(global).toContain('@import "./fonts.css";');
  expect(global).not.toMatch(/@import\s+"@fontsource-variable/);
 });

 it("declares every family the Tailwind theme asks for", () => {
  const global = readFileSync(join(STYLES, "global.css"), "utf8");
  // A renamed family in `@theme` would not error: it would silently render
  // the fallback stack, which is exactly the CLS-free but off-brand state
  // `optional` makes permanent for a page view.
  const wanted = [...global.matchAll(/--font-(?:sans|serif|mono):\s*"([^"]+)"/g)].map(
   ([, family]) => family,
  );
  const declared = new Set(local.map((face) => face.family));

  expect(wanted).toHaveLength(3);
  for (const family of wanted) expect(declared).toContain(family);
 });
});
