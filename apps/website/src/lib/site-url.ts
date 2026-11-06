/**
 * Resolves a social-card path against the site origin and refuses anything that
 * lands elsewhere.
 *
 * The override is the one value a caller can point at another host, and string
 * checks miss the forms the URL parser accepts: `/\\evil.com/x.png` and
 * `/\\t/evil.com/x.png` fail a `//` test but resolve to `https://evil.com` and
 * `https://t/evil.com`. Comparing parsed origins is the only check that holds,
 * so it lives here with tests instead of inline in the layout.
 */
export function resolveSiteUrl(value: string, siteURL: URL): URL {
  const resolved = new URL(value, siteURL);
  if (resolved.origin !== siteURL.origin) {
    throw new Error(`Expected a URL on ${siteURL.origin}, got: ${resolved.href}`);
  }
  return resolved;
}
