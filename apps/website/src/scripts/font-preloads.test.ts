// @vitest-environment node
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";

import FontPreloads from "../components/FontPreloads.astro";

const LATIN_FACES = [
 "outfit-latin-wght-normal",
 "space-grotesk-latin-wght-normal",
 "noto-sans-mono-latin-wght-normal",
];

/**
 * The preloads are what keeps `font-display: optional` (`styles/fonts.css`) from
 * pinning the fallback stack for a whole page view, so the rendered tags are
 * behaviour, not decoration: without them a cold desktop load demonstrably falls
 * back (h1 box 367 px instead of 294 px at 1350×940).
 */
describe("FontPreloads", () => {
 it("preloads exactly the three latin faces the hero needs", async () => {
  const container = await AstroContainer.create();
  const html = await container.renderToString(FontPreloads);
  const links = [...html.matchAll(/<link\b[^>]*>/g)].map(([tag]) => tag);

  expect(links).toHaveLength(LATIN_FACES.length);
  for (const face of LATIN_FACES) {
   const link = links.find((tag) => tag.includes(face));
   expect(link, `no preload for ${face}`).toBeDefined();
   // All four attributes are load-bearing: `as`/`type` let the browser apply
   // font priority, and `crossorigin` matches how a font is always fetched
   // (CORS mode) — without it the preload is discarded as a different
   // request and the face arrives late anyway.
   expect(link).toContain('rel="preload"');
   expect(link).toContain('as="font"');
   expect(link).toContain('type="font/woff2"');
   expect(link).toMatch(/crossorigin(="anonymous")?/);
   // Hashed in a build (`…-latin-wght-normal.Bc-8i84L.woff2`), bare under
   // Vitest, where the asset URL is the source path.
   expect(link).toMatch(new RegExp(`/${face}(\\.[\\w-]+)?\\.woff2"`));
  }
 });
});
