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
    ).toEqual({ key: "phc_test", host: "https://eu.i.posthog.com" });
  });

  // A half-configured build must still land on the EU host rather than the
  // posthog-js default of us.i.posthog.com, which would move visitor data to the US.
  it("falls back to the EU host when the host is absent", () => {
    expect(analytics.readConfig(newDoc("", { "data-key": "phc_test" }))).toEqual({
      key: "phc_test",
      host: analytics.DEFAULT_HOST,
    });
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
      // identify() becomes a no-op, so no persistent distinct ID is ever created.
      person_profiles: "never",
      // The site is dev-content heavy; autocapture would ship prose and code
      // snippets to PostHog.
      autocapture: false,
      capture_pageview: true,
    });
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
