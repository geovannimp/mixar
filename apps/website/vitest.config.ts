import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The analytics module is DOM-only, so the whole suite runs against jsdom.
    // The pure `resolveDestination` tests are happy there too and need no special case.
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    exclude: [
      // `stem-demo-player.test.ts` is a standalone `node:assert` script, not a
      // vitest suite — it has no describe/it, so vitest reports "No test suite
      // found". Nothing on main actually runs it either (no npm script, no moon
      // task, no CI step); it arrived from #418 with the .test.ts suffix. Excluded
      // rather than rewritten here because converting it is that PR's call.
      "**/stem-demo-player.test.ts",
      "**/node_modules/**",
    ],
  },
});
