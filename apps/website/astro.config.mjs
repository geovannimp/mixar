import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://mixat.top",
  outDir: "dist",
  vite: {
    plugins: [tailwindcss()],
  },
});
