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
              host: process.env.POSTHOG_HOST ?? "https://eu.i.posthog.com",
              sourcemaps: {
                enabled: true,
                releaseName: "mixar-website",
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
