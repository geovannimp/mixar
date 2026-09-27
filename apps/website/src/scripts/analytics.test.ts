import { beforeEach, describe, expect, it, vi } from "vitest";

// `vi.hoisted` gives the mock factory stable references, so they survive the
// `vi.resetModules()` below — otherwise each module reset would hand the test a
// fresh, unobserved `init`.
const { init, capture } = vi.hoisted(() => ({ init: vi.fn(), capture: vi.fn() }));

// The SDK sits behind a dynamic import so an unconfigured site never fetches
// the ~30KB chunk. Mocking the module lets us assert it is never reached at all.
vi.mock("posthog-js", () => ({ default: { init, capture } }));

let analytics: typeof import("./analytics");

beforeEach(async () => {
  // The module keeps an "already initialised" flag so a re-run of the layout
  // script cannot double-count pageviews. Resetting modules gives each test a
  // fresh flag instead of inheriting the previous test's.
  vi.resetModules();
  init.mockClear();
  capture.mockClear();
  document.head.innerHTML = "";
  analytics = await import("./analytics");
});

function setMeta(attrs: Record<string, string> | null) {
  document.head.innerHTML =
    attrs === null
      ? ""
      : `<meta name="mixar-analytics" ${Object.entries(attrs)
          .map(([k, v]) => `${k}="${v}"`)
          .join(" ")}>`;
}

describe("readConfig", () => {
  it("returns null when no config meta is present", () => {
    setMeta(null);
    expect(analytics.readConfig()).toBeNull();
  });

  // Local dev, fork PRs and any build without the secret must stay silent.
  it("returns null when the key is an empty string", () => {
    setMeta({ "data-key": "" });
    expect(analytics.readConfig()).toBeNull();
  });

  it("returns null when the key is only whitespace", () => {
    setMeta({ "data-key": "   " });
    expect(analytics.readConfig()).toBeNull();
  });

  it("returns null when the meta carries no key at all", () => {
    setMeta({ "data-host": analytics.DEFAULT_HOST });
    expect(analytics.readConfig()).toBeNull();
  });

  it("reads the key and host from the meta", () => {
    setMeta({ "data-key": "phc_test", "data-host": "https://eu.i.posthog.com" });
    expect(analytics.readConfig()).toEqual({
      key: "phc_test",
      host: "https://eu.i.posthog.com",
    });
  });

  // A half-configured build must still land on the EU host rather than the
  // posthog-js default of us.i.posthog.com, which would move visitor data to the US.
  it("falls back to the EU host when the host is absent", () => {
    setMeta({ "data-key": "phc_test" });
    expect(analytics.readConfig()).toEqual({
      key: "phc_test",
      host: analytics.DEFAULT_HOST,
    });
  });
});

describe("initAnalytics", () => {
  it("does not load the SDK when no key is configured", async () => {
    setMeta(null);
    await analytics.initAnalytics();
    expect(init).not.toHaveBeenCalled();
  });

  it("does not load the SDK when the key is blank", async () => {
    setMeta({ "data-key": "" });
    await analytics.initAnalytics();
    expect(init).not.toHaveBeenCalled();
  });

  it("initializes with cookieless mode and no autocapture", async () => {
    setMeta({ "data-key": "phc_test", "data-host": "https://eu.i.posthog.com" });
    await analytics.initAnalytics();

    expect(init).toHaveBeenCalledTimes(1);
    expect(init).toHaveBeenCalledWith("phc_test", {
      api_host: "https://eu.i.posthog.com",
      // No cookie, no localStorage, no consent banner.
      cookieless_mode: "always",
      // identify() becomes a no-op, so no persistent distinct ID is ever created.
      person_profiles: "never",
      // The site is dev-content heavy; autocapture would ship prose and code
      // snippets to PostHog.
      autocapture: false,
      capture_pageview: true,
    });
  });

  it("defaults to the EU host", async () => {
    setMeta({ "data-key": "phc_test" });
    await analytics.initAnalytics();
    expect(init.mock.calls[0]?.[1]).toMatchObject({ api_host: analytics.DEFAULT_HOST });
  });

  // A double init double-counts every pageview, which corrupts the funnel
  // denominator. Astro can re-run a layout script; this must be idempotent.
  it("does not initialize twice when called repeatedly", async () => {
    setMeta({ "data-key": "phc_test" });
    await analytics.initAnalytics();
    await analytics.initAnalytics();
    expect(init).toHaveBeenCalledTimes(1);
  });

  // Removing the `.catch()` makes this reject, so it genuinely guards the
  // "a bad key must not throw into page scripts" requirement.
  it("swallows an SDK error instead of rejecting", async () => {
    init.mockImplementationOnce(() => {
      throw new Error("bad project key");
    });
    setMeta({ "data-key": "phc_test" });
    await expect(analytics.initAnalytics()).resolves.toBeUndefined();
  });
});
