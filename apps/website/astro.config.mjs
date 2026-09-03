import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

const pagesBase = process.env.GITHUB_PAGES_BASE;

export default defineConfig({
  site: pagesBase ? "https://geovannimp.github.io" : "https://mixar.app",
  base: pagesBase ?? "/",
  outDir: "dist",
  vite: {
    plugins: [tailwindcss()],
  },
});
