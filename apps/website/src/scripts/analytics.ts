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
  /**
   * Off unless the build opts in. A wrong host, a bad token or a request blocked
   * by a tracking blocker all fail silently, so local setup needs a way to see
   * what the SDK is actually doing.
   */
  debug: boolean;
};

/** PostHog's `[object] [verb]` convention. */
const OUTBOUND_CLICK_EVENT = "outbound link clicked";

/**
 * Losing a conversion because an annotation was forgotten is worse than an
 * `unknown` bucket, which at least keeps the event in the destination funnel.
 * A *misspelled* annotation is the case this does not help: it is non-empty, so
 * it would fork a real bucket in two. `PLACEMENTS` closes that set and the
 * component sources are checked against it in `analytics-events.test.ts`.
 */
const UNKNOWN_PLACEMENT = "unknown";

type Capture = (event: string, properties: Record<string, string>) => void;

/**
 * Properties posthog-js attaches to every event by default, removed before
 * anything leaves the page.
 *
 * `$timezone` and `$browser_language` are a coarse location signal, which would
 * contradict the privacy page's claim that no location data is collected. They
 * are safe to remove: they are not inputs to the cookieless hash.
 *
 * `$raw_user_agent` is deliberately NOT in this list. Cookieless mode derives
 * the anonymous distinct id server-side from
 * `hash(calendar day + user agent + IP + host)`, so stripping it makes the
 * identity uncomputable and ingestion discards every event with
 * `cookieless_missing_user_agent` — behind a 200 OK, so it looks like success.
 * The agent string is an input to a one-way hash whose salt is discarded daily,
 * not a stored property, which is why keeping it is compatible with the
 * privacy claim on the privacy page.
 *
 * Screen dimensions are also left alone: not identifying, and useful for
 * catching a broken responsive layout.
 */
const REDACTED_PROPERTIES = [
  "$timezone",
  "$timezone_offset",
  "$browser_language",
  "$browser_language_prefix",
] as const;

/**
 * `before_send` hook. Mutates and returns the event, or passes `null` through
 * so a drop decision made elsewhere is preserved.
 */
export function redactEvent<T extends { properties?: Record<string, unknown> } | null>(
  event: T,
): T {
  if (event === null || event.properties === undefined) return event;
  for (const key of REDACTED_PROPERTIES) {
    delete event.properties[key];
  }
  return event;
}

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
  return {
    key,
    host: host === "" ? DEFAULT_HOST : host,
    debug: meta.dataset.debug === "true",
  };
}

export function initAnalytics(doc: Document = document): Promise<void> {
  const config = readConfig(doc);
  if (config === null || doc.documentElement.hasAttribute(STARTED_ATTR)) {
    return Promise.resolve();
  }
  doc.documentElement.setAttribute(STARTED_ATTR, "");

  // Listen immediately and buffer until the SDK is ready. The posthog chunk is
  // ~95KB gzipped, and the hero CTA is the first thing a visitor touches — with
  // no buffer, clicks in the first moments of page life are dropped silently
  // and the funnel reads low, biased against exactly the fast clickers it
  // exists to measure.
  const pending: Array<[string, Record<string, string>]> = [];
  let send: Capture | null = null;
  const capture: Capture = (name, properties) => {
    if (send === null) {
      pending.push([name, properties]);
      return;
    }
    send(name, properties);
  };
  trackOutboundClicks(capture, doc);

  return import("posthog-js")
    .then(({ default: posthog }) => {
      posthog.init(config.key, {
        api_host: config.host,
        // No cookie, no localStorage, no consent banner. PostHog counts unique
        // users with a daily-salted server-side hash instead.
        cookieless_mode: "always",
        // NOTE: `person_profiles: 'never'` is deliberately NOT set. It stamps
        // every event with $process_person_profile: false, and PostHog's
        // cookieless pipeline assigns the server-side hashed distinct id during
        // person processing — so it can leave an event with no distinct_id at
        // all, which ingestion then discards behind a 200 OK. This site never
        // calls identify(), and cookieless mode already prevents a persistent
        // ID, so the guard bought nothing. See the privacy note in
        // src/pages/privacy.astro before changing this.
        // The site is dev-content heavy; autocapture would ship prose, code
        // snippets and MIDI mapping tables to PostHog.
        autocapture: false,
        capture_pageview: true,
        // Both default to false, so a posthog-js upgrade could start
        // auto-loading a survey banner on a site whose entire privacy story is
        // "there is nothing to consent to".
        disable_surveys: true,
        advanced_disable_flags: true,
        // Unhandled errors and unhandled rejections. `capture_exceptions` is a
        // separate option from `autocapture` — resolved from the SDK, not the
        // docs, which lead with `error_tracking` (that one has no on/off flag;
        // it configures suppression rules). Left undefined it resolves to all
        // false, so error tracking is off unless this is set. Console errors
        // stay off: third-party noise buries real ones.
        capture_exceptions: true,
        before_send: redactEvent,
        // Defaults to false, which would leave a visitor who sends Do Not Track
        // being tracked and make the privacy page's promise false. When true the
        // SDK also honours `msDoNotTrack` and Global Privacy Control, so this is
        // broader than the DNT the page names.
        respect_dnt: true,
        // Drops exceptions raised by browser extensions and other injected
        // scripts before capture. The privacy page states this, so it is set
        // explicitly rather than left as an SDK default.
        error_tracking: { captureExtensionExceptions: false },
        debug: config.debug,
      });

      send = (name, properties) => posthog.capture(name, properties);
      for (const [name, properties] of pending) send(name, properties);
      pending.length = 0;

      // The snippet-based integrations install a `posthog` global; bundling the
      // SDK as a module does not. Expose it under debug so the console is
      // actually usable for `posthog.capture(...)` and config inspection —
      // otherwise a debug switch that gives you nothing to look at.
      if (config.debug) {
        (globalThis as { posthog?: unknown }).posthog = posthog;
      }
    })
    .catch(() => {
      // Analytics must never break the page. A failed SDK load is not the
      // visitor's problem and is not worth a console error. The buffer is
      // dropped with it: there is nothing to flush it into.
      pending.length = 0;
    });
}
