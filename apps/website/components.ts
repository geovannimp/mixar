import { defineComponents } from "blume";

/**
 * Blume builds the docs chrome itself, so the pieces of the site's own layout
 * that the docs pages still need are injected here.
 */
export default defineComponents({
  layout: {
    // The docs pages carry the site's analytics, the same way the marketing
    // pages do through `BaseLayout`: a config meta tag the script reads, and
    // the script that starts the SDK. `PageFooter` is an injection point with
    // no built-in, so nothing of Blume's chrome is replaced.
    PageFooter: "./src/components/docs/Analytics.astro",
  },
});
