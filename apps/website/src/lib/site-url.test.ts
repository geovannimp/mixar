import { describe, expect, it } from "vitest";

import { resolveSiteUrl } from "./site-url";

const siteURL = new URL("https://mixar.top");

describe("resolveSiteUrl", () => {
  it("resolves a site-root path against the site origin", () => {
    expect(resolveSiteUrl("/mixar-banner.png", siteURL).href).toBe(
      "https://mixar.top/mixar-banner.png",
    );
  });

  it("keeps a same-origin absolute URL", () => {
    expect(resolveSiteUrl("https://mixar.top/_astro/card.jpg", siteURL).href).toBe(
      "https://mixar.top/_astro/card.jpg",
    );
  });

  // Every form below fails a `//` prefix test but resolves off-origin.
  it.each([
    ["protocol-relative", "//evil.com/x.png"],
    ["backslash", "/\\evil.com/x.png"],
    ["backslash after segment", "/\\t/evil.com/x.png"],
    ["absolute", "https://evil.com/x.png"],
  ])("rejects the %s form", (_label, value) => {
    expect(() => resolveSiteUrl(value, siteURL)).toThrow(/must stay on|Expected a URL on/);
  });
});
