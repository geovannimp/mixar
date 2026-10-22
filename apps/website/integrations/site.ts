import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { AstroIntegration } from "astro";
import posthogRollup from "@posthog/rollup-plugin";

/**
 * Blume generates the Astro project it builds from, so the site's own Astro
 * settings have to be expressed as an integration that hooks into that
 * generated config. These are the settings Blume has no config knob for.
 */

/** The site's own project root — Blume's generated runtime lives in `.blume/`. */
const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * Source map upload needs a *personal* API key, which only CI has — the public
 * `phc_` project token used by the browser cannot write symbol sets. Without
 * these the site still builds; error tracking just gets minified stack traces.
 */
const posthogApiKey = process.env.POSTHOG_API_KEY ?? "";
const posthogProjectId = process.env.POSTHOG_PROJECT_ID ?? "";
const uploadSourceMaps = posthogApiKey !== "" && posthogProjectId !== "";

export default function site(): AstroIntegration {
  return {
    name: "mixar-site",
    hooks: {
      "astro:config:setup": ({ updateConfig }) => {
        updateConfig({
          // `directory` output would still be 301'd to a trailing slash by
          // GitHub Pages; `file` emits `foo.html`, which Pages resolves
          // directly at `/foo`. The trailing-slash-redirects integration
          // re-adds a redirect stub at each `/foo/` so pre-existing slashed
          // URLs keep working.
          build: { format: "file" },
          vite: {
            // Blume's runtime lives in `.blume/`, so Vite would look for the
            // `.env` file there. The site's env file sits at the project root.
            envDir: projectRoot,
            plugins: [
              // Dev-safe: the plugin uploads from `writeBundle`, which Vite's
              // dev server never calls, so this is inert under `blume dev`.
              ...(uploadSourceMaps
                ? [
                    posthogRollup({
                      personalApiKey: posthogApiKey,
                      projectId: posthogProjectId,
                      // The APP host, which the source map upload API uses —
                      // not the `eu.i.` ingestion host that posthog-js uses
                      // for events. The two are different and conflating them
                      // makes local uploads 404.
                      host: process.env.POSTHOG_HOST ?? "https://eu.posthog.com",
                      sourcemaps: {
                        enabled: true,
                        // The default releaseMode is `event`, which resolves a
                        // release id during the build and injects it into
                        // every chunk as `globalThis._posthogReleaseId`;
                        // posthog-js reads that when capturing $exception and
                        // tags it with $release_id. So the exceptions and the
                        // uploaded maps are linked by the build itself.
                        releaseName:
                          process.env.POSTHOG_RELEASE_NAME ?? "mixar-website",
                        // Emits maps as Rollup `hidden` (not referenced by a
                        // sourceMappingURL comment) and deletes them after
                        // upload, so the full original source is never served
                        // publicly.
                        deleteAfterUpload: true,
                      },
                    }),
                  ]
                : []),
            ],
          },
        });
      },
    },
  };
}
