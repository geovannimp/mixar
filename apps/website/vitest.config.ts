import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The analytics module is DOM-only, so the whole suite runs against jsdom.
    // The pure `resolveDestination` tests are happy there too and need no special case.
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
