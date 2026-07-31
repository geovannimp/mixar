import { defineConfig } from "oxlint";

export default defineConfig({
  options: {
    typeAware: true,
    typeCheck: true,
  },
  plugins: ["typescript", "unicorn", "oxc", "import"],
  categories: {
    correctness: "error",
  },
  rules: {
    "import/no-relative-parent-imports": "error",
  },
  env: {
    builtin: true,
  },
  ignorePatterns: ["dist/**", "node_modules/**", "src-tauri/target/**"],
});
