import { defineConfig } from "astro/config";
import mdx from "@astrojs/mdx";
import tailwindcss from "@tailwindcss/vite";
import posthogRollup from "@posthog/rollup-plugin";

// Source map upload needs a *personal* API key, which only CI has — the public
// `phc_` project token used by the browser cannot write symbol sets. Without
// these the site still builds; error tracking just gets minified stack traces.
const posthogApiKey = process.env.POSTHOG_API_KEY;
const posthogProjectId = process.env.POSTHOG_PROJECT_ID;
const uploadSourceMaps = Boolean(posthogApiKey && posthogProjectId);

export default defineConfig({
  site: "https://mixar.top",
  outDir: "dist",
  integrations: [mdx()],
  markdown: {
    // Dual themes emit both `--shiki-light*` and `--shiki-dark*` custom
    // properties; the dark set is activated from DocsLayout because Astro does
    // not inject the `prefers-color-scheme` override itself, and this site
    // toggles `data-theme` rather than the OS preference.
    shikiConfig: {
      themes: { light: "github-light", dark: "github-dark" },
      // Shiki has no Rhai grammar; Rust highlights its `fn`/`let`/`const`/`//`
      // syntax closely enough, and the alias avoids a plaintext fallback warning.
      langAlias: { rhai: "rust" },
    },
  },
  vite: {
    plugins: [
      tailwindcss(),
      // Dev-safe: the plugin uploads from `writeBundle`, which Vite's dev
      // server never calls, so this is inert under `astro dev`.
      ...(uploadSourceMaps
        ? [
            posthogRollup({
              personalApiKey: posthogApiKey,
              projectId: posthogProjectId,
              // The APP host, which the source map upload API uses — not the
              // `eu.i.` ingestion host that posthog-js uses for events. The two
              // are different and conflating them makes local uploads 404.
              host: process.env.POSTHOG_HOST ?? "https://eu.posthog.com",
              sourcemaps: {
                enabled: true,
                // The default releaseMode is `event`, which resolves a release
                // id during the build and injects it into every chunk as
                // `globalThis._posthogReleaseId`; posthog-js reads that when
                // capturing $exception and tags it with $release_id. So the
                // exceptions and the uploaded maps are linked by the build
                // itself — nothing here has to be kept in sync by hand.
                // Env-overridable so a rename needs no code edit.
                releaseName: process.env.POSTHOG_RELEASE_NAME ?? "mixar-website",
                // Emits maps as Rollup `hidden` (not referenced by a
                // sourceMappingURL comment) and deletes them after upload, so
                // the full original source is never served publicly.
                deleteAfterUpload: true,
              },
            }),
          ]
        : []),
    ],
  },
});
