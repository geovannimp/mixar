import { defineConfig } from "blume";

import site from "./integrations/site";
import trailingSlashRedirects from "./integrations/trailing-slash-redirects";

/**
 * Blume drives the whole site: the docs content, the marketing pages under
 * `src/pages`, and the agent-facing surfaces (llms.txt, `.md` mirrors, the
 * JSON API, the agent skill).
 *
 * `basePath` mounts the docs under `/docs` while the site root stays ours, so
 * the marketing pages keep their current routes.
 */

/**
 * The site's own typefaces, taken from the `@fontsource-variable` packages
 * the repo already installs — the same files the marketing pages load, so
 * both halves of the site render one typeface. Blume's default is a
 * build-time download from Google Fonts; pointing the roles at local files
 * keeps the build offline and the fonts self-hosted.
 */
const fonts = {
  display: {
    name: "Space Grotesk Variable",
    variants: [
      {
        src: "../../node_modules/@fontsource-variable/space-grotesk/files/space-grotesk-latin-wght-normal.woff2",
        weight: "300..700",
      },
    ],
  },
  body: {
    name: "Outfit Variable",
    variants: [
      {
        src: "../../node_modules/@fontsource-variable/outfit/files/outfit-latin-wght-normal.woff2",
        weight: "100..900",
      },
    ],
  },
  mono: {
    name: "Noto Sans Mono Variable",
    variants: [
      {
        src: "../../node_modules/@fontsource-variable/noto-sans-mono/files/noto-sans-mono-latin-wght-normal.woff2",
        weight: "100..900",
      },
    ],
  },
};

export default defineConfig({
  title: "Mixar",
  description:
    "Mixar documentation for developers and users: MIDI controller mappings, the mapping bundle format, and more.",
  logo: "/mixar-logo.png",
  content: {
    root: "src/content/docs",
    // The marketing pages live next to the site's own components; Blume
    // mounts each file under this folder as a route of its own.
    pages: "src/pages",
  },
  basePath: "/docs",
  theme: {
    accent: { light: "#0f766e", dark: "#2dd4bf" },
    radius: "none",
    mode: "dark",
    toggle: true,
    fonts,
  },
  markdown: {
    code: {
      icons: true,
      theme: { light: "github-light", dark: "github-dark" },
    },
  },
  agents: {
    llmsTxt: true,
    catalog: true,
    skillMd: true,
    // The MCP server is a live endpoint, so it needs server output — the site
    // deploys to GitHub Pages as static files.
    mcp: { enabled: false },
  },
  seo: {
    // The generated per-page card is off: its font subsetting needs a Takumi
    // release Blume 2.2.4 is not built against. The site's own banner is the
    // social image, the way the docs pages carried it before.
    og: { enabled: false, image: "/mixar-banner.png" },
    sitemap: true,
    robots: true,
    structuredData: true,
  },
  deployment: {
    site: "https://mixar.top",
  },
  // The site's own Astro settings — the GitHub Pages URL shape, the `.env`
  // location, and the source map upload — which Blume has no config knob for.
  integrations: [site(), trailingSlashRedirects()],
});
