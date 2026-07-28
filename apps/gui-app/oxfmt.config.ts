import { defineConfig } from "oxfmt";

export default defineConfig({
  ignorePatterns: ["dist/**", "node_modules/**", "src-tauri/target/**", "package-lock.json"],
});
