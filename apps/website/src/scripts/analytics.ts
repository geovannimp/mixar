import { resolveDestination } from "./analytics-events";

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

/** PostHog's `[object] [verb]` convention. */
const OUTBOUND_CLICK_EVENT = "outbound link clicked";

/**
 * Losing a conversion because an annotation was forgotten is worse than an
 * `unknown` bucket, which at least keeps the event in the destination funnel.
 */
const UNKNOWN_PLACEMENT = "unknown";

type Capture = (event: string, properties: Record<string, string>) => void;

/**
 * One delegated listener for the whole page. CTAs are annotated declaratively
 * with `data-placement`; `destination` is derived from the href, because every
 * outbound CTA href comes from `src/consts.ts`. Links that are not ours — the
 * GPL text link, internal navigation — resolve to null and are never captured.
 */
function trackOutboundClicks(capture: Capture, doc: Document): void {
  doc.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    // Button.astro renders its label through a slot, so the click target is
    // usually a child of the anchor rather than the anchor itself.
    const anchor = target.closest("a[href]");
    if (anchor === null) return;

    const destination = resolveDestination(anchor.getAttribute("href") ?? "");
    if (destination === null) return;

    capture(OUTBOUND_CLICK_EVENT, {
      destination,
      placement: anchor.getAttribute("data-placement")?.trim() || UNKNOWN_PLACEMENT,
    });
  });
}

/**
 * "Already initialised" is a property of the page, not of the module, so the
 * guard is recorded on the document. A module-level flag would also make the
 * function single-shot across documents, which is wrong for anything that
 * renders more than one.
 */
const STARTED_ATTR = "data-mixar-analytics-started";

export function readConfig(doc: Document = document): AnalyticsConfig | null {
  const meta = doc.querySelector<HTMLMetaElement>(`meta[name="${META_NAME}"]`);
  if (meta === null) return null;

  const key = meta.dataset.key?.trim() ?? "";
  if (key === "") return null;

  const host = meta.dataset.host?.trim() ?? "";
  return { key, host: host === "" ? DEFAULT_HOST : host };
}

export function initAnalytics(doc: Document = document): Promise<void> {
  const config = readConfig(doc);
  if (config === null || doc.documentElement.hasAttribute(STARTED_ATTR)) {
    return Promise.resolve();
  }
  doc.documentElement.setAttribute(STARTED_ATTR, "");

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

      trackOutboundClicks((name, properties) => posthog.capture(name, properties), doc);
    })
    .catch(() => {
      // Analytics must never break the page. A failed SDK load is not the
      // visitor's problem and is not worth a console error.
    });
}
