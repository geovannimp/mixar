import { beforeEach, describe, expect, it, vi } from "vitest";

// `vi.hoisted` gives the mock factory stable references, so they survive the
// `vi.resetModules()` below — otherwise each module reset would hand the test a
// fresh, unobserved `init`.
const { init, capture } = vi.hoisted(() => ({ init: vi.fn(), capture: vi.fn() }));

// The SDK sits behind a dynamic import so an unconfigured site never fetches
// the ~30KB chunk. Mocking the module lets us assert it is never reached at all.
vi.mock("posthog-js", () => ({ default: { init, capture } }));

import {
  CONTRIBUTING_URL,
  GITHUB_REPO,
  QUICK_START_URL,
  README_URL,
  TECH_SPEC_URL,
} from "../consts";

let analytics: typeof import("./analytics");

beforeEach(async () => {
  vi.resetModules();
  init.mockClear();
  capture.mockClear();
  analytics = await import("./analytics");
});

/**
 * A fresh document per test. The click listener is delegated onto the document
 * and the init guard is recorded on the document, and `document` outlives an
 * individual test — so reusing it would leave earlier tests' listeners attached
 * and double-count every capture.
 */
function newDoc(body = "", meta: Record<string, string> | null = null) {
  const doc = document.implementation.createHTMLDocument("test");
  doc.head.innerHTML =
    meta === null
      ? ""
      : `<meta name="mixar-analytics" ${Object.entries(meta)
          .map(([k, v]) => `${k}="${v}"`)
          .join(" ")}>`;
  doc.body.innerHTML = body;
  return doc;
}

const HOSTED = { "data-key": "phc_test" };

describe("readConfig", () => {
  it("returns null when no config meta is present", () => {
    expect(analytics.readConfig(newDoc())).toBeNull();
  });

  // Local dev, fork PRs and any build without the secret must stay silent.
  it("returns null when the key is an empty string", () => {
    expect(analytics.readConfig(newDoc("", { "data-key": "" }))).toBeNull();
  });

  it("returns null when the key is only whitespace", () => {
    expect(analytics.readConfig(newDoc("", { "data-key": "   " }))).toBeNull();
  });

  it("returns null when the meta carries no key at all", () => {
    expect(
      analytics.readConfig(newDoc("", { "data-host": "https://eu.i.posthog.com" })),
    ).toBeNull();
  });

  it("reads the key and host from the meta", () => {
    expect(
      analytics.readConfig(
        newDoc("", { "data-key": "phc_test", "data-host": "https://eu.i.posthog.com" }),
      ),
    ).toEqual({ key: "phc_test", host: "https://eu.i.posthog.com", debug: false });
  });

  // A half-configured build must still land on the EU host rather than the
  // posthog-js default of us.i.posthog.com, which would move visitor data to the US.
  it("falls back to the EU host when the host is absent", () => {
    expect(analytics.readConfig(newDoc("", { "data-key": "phc_test" }))).toEqual({
      key: "phc_test",
      host: analytics.DEFAULT_HOST,
      debug: false,
    });
  });

  // A wrong host, a bad token or a blocked request all fail silently otherwise,
  // so local setup needs a way to see what the SDK is actually doing.
  it("opts into SDK debug logging when the meta asks for it", () => {
    expect(
      analytics.readConfig(newDoc("", { "data-key": "phc_test", "data-debug": "true" })),
    ).toMatchObject({ debug: true });
  });

  it("leaves debug off for any other value", () => {
    expect(
      analytics.readConfig(newDoc("", { "data-key": "phc_test", "data-debug": "1" })),
    ).toMatchObject({ debug: false });
  });
});

describe("initAnalytics", () => {
  it("does not load the SDK when no key is configured", async () => {
    await analytics.initAnalytics(newDoc());
    expect(init).not.toHaveBeenCalled();
  });

  it("does not load the SDK when the key is blank", async () => {
    await analytics.initAnalytics(newDoc("", { "data-key": "" }));
    expect(init).not.toHaveBeenCalled();
  });

  it("initializes with cookieless mode and no autocapture", async () => {
    await analytics.initAnalytics(newDoc("", HOSTED));

    expect(init).toHaveBeenCalledTimes(1);
    expect(init).toHaveBeenCalledWith("phc_test", {
      api_host: analytics.DEFAULT_HOST,
      // No cookie, no localStorage, no consent banner.
      cookieless_mode: "always",
      // The site is dev-content heavy; autocapture would ship prose and code
      // snippets to PostHog.
      autocapture: false,
      capture_pageview: true,
      // Observed SDK defaults, not guarantees: both default to false, so a
      // posthog-js upgrade could start auto-loading a survey banner on a site
      // whose whole privacy story is "there is nothing to consent to".
      disable_surveys: true,
      advanced_disable_flags: true,
      // Strips the location-ish and fingerprinting properties the SDK attaches
      // by default, so the privacy page can be exact about what leaves.
      before_send: expect.any(Function),
      debug: false,
    });
  });

  it("forwards the debug opt-in to the SDK", async () => {
    await analytics.initAnalytics(
      newDoc("", { "data-key": "phc_test", "data-debug": "true" }),
    );
    expect(init.mock.calls[0]?.[1]).toMatchObject({ debug: true });
  });

  // person_profiles: 'never' stamps every event with
  // $process_person_profile: false, and PostHog's cookieless pipeline assigns
  // the server-side hashed distinct id during person processing — so the flag
  // can leave the event with no distinct_id at all, which ingestion then drops
  // behind a 200 OK. The site never calls identify(), and cookieless mode
  // already prevents a persistent ID, so the guard bought nothing.
  it("does not disable person profiles", async () => {
    await analytics.initAnalytics(newDoc("", HOSTED));
    expect(init.mock.calls[0]?.[1]).not.toHaveProperty("person_profiles");
  });

  // Without a global there is nothing to poke at from the console, which is the
  // entire point of the debug switch.
  it("exposes the SDK on window when debug is on", async () => {
    await analytics.initAnalytics(
      newDoc("", { "data-key": "phc_test", "data-debug": "true" }),
    );
    expect((globalThis as { posthog?: unknown }).posthog).toBeDefined();
  });

  it("keeps the SDK off window when debug is off", async () => {
    delete (globalThis as { posthog?: unknown }).posthog;
    await analytics.initAnalytics(newDoc("", HOSTED));
    expect((globalThis as { posthog?: unknown }).posthog).toBeUndefined();
  });

  // The SDK is ~95KB gzipped. Without buffering, the hero CTA — the first thing
  // a visitor touches — is missed whenever they click before the chunk resolves,
  // and the loss is silent and biased toward fast clickers.
  it("captures a click that lands before the SDK has loaded", async () => {
    const doc = newDoc(
      `<a id="cta" href="${GITHUB_REPO}" data-placement="hero">go</a>`,
      HOSTED,
    );
    // Deliberately not awaited: the click happens while the chunk is in flight.
    const ready = analytics.initAnalytics(doc);
    doc.getElementById("cta")?.click();
    await ready;

    expect(capture).toHaveBeenCalledWith("outbound link clicked", {
      destination: "repo",
      placement: "hero",
    });
  });

  it("does not capture a buffered click twice", async () => {
    const doc = newDoc(
      `<a id="cta" href="${GITHUB_REPO}" data-placement="hero">go</a>`,
      HOSTED,
    );
    const ready = analytics.initAnalytics(doc);
    doc.getElementById("cta")?.click();
    await ready;
    doc.getElementById("cta")?.click();

    expect(capture).toHaveBeenCalledTimes(2);
  });

  // A double init double-counts every pageview, which corrupts the funnel
  // denominator. Astro can re-run a layout script; this must be idempotent.
  it("does not initialize twice on the same document", async () => {
    const doc = newDoc("", HOSTED);
    await analytics.initAnalytics(doc);
    await analytics.initAnalytics(doc);
    expect(init).toHaveBeenCalledTimes(1);
  });

  // Removing the `.catch()` makes this reject, so it genuinely guards the
  // "a bad key must not throw into page scripts" requirement.
  it("swallows an SDK error instead of rejecting", async () => {
    init.mockImplementationOnce(() => {
      throw new Error("bad project key");
    });
    await expect(analytics.initAnalytics(newDoc("", HOSTED))).resolves.toBeUndefined();
  });
});

describe("outbound link tracking", () => {
  async function boot(body: string) {
    const doc = newDoc(body, HOSTED);
    await analytics.initAnalytics(doc);
    return doc;
  }

  it("captures a click on an annotated repository link", async () => {
    const doc = await boot(
      `<a id="cta" href="${GITHUB_REPO}" data-placement="hero">Star on GitHub</a>`,
    );
    doc.getElementById("cta")?.click();

    expect(capture).toHaveBeenCalledWith("outbound link clicked", {
      destination: "repo",
      placement: "hero",
    });
  });

  it("captures each closed destination", async () => {
    const cases = [
      [GITHUB_REPO, "repo"],
      [QUICK_START_URL, "quick_start"],
      [README_URL, "readme"],
      [TECH_SPEC_URL, "tech_spec"],
      [CONTRIBUTING_URL, "contributing"],
    ] as const;

    for (const [href, destination] of cases) {
      capture.mockClear();
      const doc = await boot(`<a id="cta" href="${href}" data-placement="footer">go</a>`);
      doc.getElementById("cta")?.click();
      expect(capture).toHaveBeenCalledWith("outbound link clicked", {
        destination,
        placement: "footer",
      });
    }
  });

  // Footer.astro links to the GPL text. Counting it would inflate the funnel's
  // conversion rate with a licensing link.
  it("ignores a non-repository external link", async () => {
    const doc = await boot(
      `<a id="cta" href="https://www.gnu.org/licenses/gpl-3.0.html" data-placement="footer">GPL</a>`,
    );
    doc.getElementById("cta")?.click();
    expect(capture).not.toHaveBeenCalled();
  });

  it("ignores an internal link", async () => {
    const doc = await boot(`<a id="cta" href="/developers" data-placement="header">Devs</a>`);
    doc.getElementById("cta")?.click();
    expect(capture).not.toHaveBeenCalled();
  });

  // Losing a conversion because an annotation was forgotten is worse than an
  // `unknown` bucket, which at least keeps the event in the destination funnel.
  it("falls back to an unknown placement when the annotation is missing", async () => {
    const doc = await boot(`<a id="cta" href="${GITHUB_REPO}">Star on GitHub</a>`);
    doc.getElementById("cta")?.click();

    expect(capture).toHaveBeenCalledWith("outbound link clicked", {
      destination: "repo",
      placement: "unknown",
    });
  });

  // Button.astro renders its label through a slot, so the click target is almost
  // never the anchor itself.
  it("resolves the anchor when the click lands on a child element", async () => {
    const doc = await boot(
      `<a id="cta" href="${QUICK_START_URL}" data-placement="hero"><span id="label">Quick start</span></a>`,
    );
    doc.getElementById("label")?.click();

    expect(capture).toHaveBeenCalledWith("outbound link clicked", {
      destination: "quick_start",
      placement: "hero",
    });
  });

  it("does not throw when the click target is not an element", async () => {
    const doc = await boot(`<a id="cta" href="${GITHUB_REPO}" data-placement="hero">go</a>`);
    expect(() => doc.dispatchEvent(new MouseEvent("click", { bubbles: true }))).not.toThrow();
  });

  it("installs no listener when analytics is unconfigured", async () => {
    const doc = newDoc(`<a id="cta" href="${GITHUB_REPO}" data-placement="hero">go</a>`);
    await analytics.initAnalytics(doc);
    doc.getElementById("cta")?.click();

    expect(init).not.toHaveBeenCalled();
    expect(capture).not.toHaveBeenCalled();
  });
});

describe("redactEvent", () => {
  // posthog-js attaches these to every event by default. $timezone and
  // $browser_language are a coarse location signal, which would contradict the
  // privacy page's claim that no location data is collected.
  it("strips the timezone properties", () => {
    const event = { properties: { $timezone: "America/Sao_Paulo", $timezone_offset: -180 } };
    expect(analytics.redactEvent(event).properties).toEqual({});
  });

  it("strips the browser language properties", () => {
    const event = { properties: { $browser_language: "pt-BR", $browser_language_prefix: "pt" } };
    expect(analytics.redactEvent(event).properties).toEqual({});
  });

  // Do NOT strip $raw_user_agent. Cookieless mode hashes
  // calendar day + user agent + IP + host server-side to build the anonymous
  // distinct id, so removing it makes the identity uncomputable and ingestion
  // drops every event with `cookieless_missing_user_agent` — silently, behind
  // a 200 OK. The agent string is an input to a one-way daily-rotating hash,
  // not a stored property.
  it("keeps the raw user agent, which cookieless hashing requires", () => {
    const event = { properties: { $raw_user_agent: "Mozilla/5.0 …", $timezone: "UTC" } };
    expect(analytics.redactEvent(event).properties).toEqual({
      $raw_user_agent: "Mozilla/5.0 …",
    });
  });

  it("keeps the fields the funnel is built on", () => {
    const event = {
      properties: {
        destination: "repo",
        placement: "hero",
        $current_url: "https://mixar.top/",
        $pathname: "/",
        $host: "mixar.top",
        title: "Mixar | Free DJ software",
      },
    };
    expect(analytics.redactEvent(event).properties).toEqual(event.properties);
  });

  it("keeps screen dimensions, which are not identifying", () => {
    const event = { properties: { $screen_width: 1920, $viewport_height: 1080 } };
    expect(analytics.redactEvent(event).properties).toEqual(event.properties);
  });

  // `null` is how before_send drops an event; it must pass through untouched.
  it("passes null through so a drop decision is preserved", () => {
    expect(analytics.redactEvent(null)).toBeNull();
  });

  it("passes an event with no properties through", () => {
    const event = {};
    expect(analytics.redactEvent(event)).toEqual({});
  });
});
