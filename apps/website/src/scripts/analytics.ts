/** Name of the `<meta>` element BaseLayout.astro renders when analytics is configured. */
export const META_NAME = "mixar-analytics";

/**
 * EU by default. posthog-js falls back to `us.i.posthog.com` when `api_host` is
 * unset, so a half-configured build would silently move visitor data to the US.
 */
export const DEFAULT_HOST = "https://eu.i.posthog.com";

export type AnalyticsConfig = {
  key: string;
  host: string;
};

/** A re-run of the layout script must not double-count pageviews. */
let started = false;

export function readConfig(doc: Document = document): AnalyticsConfig | null {
  const meta = doc.querySelector<HTMLMetaElement>(`meta[name="${META_NAME}"]`);
  const key = meta?.dataset.key?.trim() ?? "";
  if (key === "") return null;

  const host = meta.dataset.host?.trim() ?? "";
  return { key, host: host === "" ? DEFAULT_HOST : host };
}

export function initAnalytics(doc: Document = document): Promise<void> {
  const config = readConfig(doc);
  if (config === null || started) return Promise.resolve();
  started = true;

  return import("posthog-js")
    .then(({ default: posthog }) => {
      posthog.init(config.key, {
        api_host: config.host,
        // No cookie, no localStorage, no consent banner. PostHog counts unique
        // users with a daily-salted server-side hash instead.
        cookieless_mode: "always",
        // identify() becomes a no-op, so no persistent distinct ID is created.
        person_profiles: "never",
        // The site is dev-content heavy; autocapture would ship prose, code
        // snippets and MIDI mapping tables to PostHog.
        autocapture: false,
        capture_pageview: true,
      });
    })
    .catch(() => {
      // Analytics must never break the page. A failed SDK load is not the
      // visitor's problem and is not worth a console error.
    });
}
