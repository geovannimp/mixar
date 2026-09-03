import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://mixar.top",
  outDir: "dist",
  vite: {
    plugins: [tailwindcss()],
  },
});
