import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://mixar.app",
  base: "/",
  outDir: "dist",
  vite: {
    plugins: [tailwindcss()],
  },
});
