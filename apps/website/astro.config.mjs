import { defineConfig } from "astro/config";
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
                // Must match the release naming in PostHog or uploads succeed
                // but stop matching, and traces silently go back to minified.
                // Env-overridable so it can be changed without a code edit.
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
