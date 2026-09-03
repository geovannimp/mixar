export const GITHUB_REPO = "https://github.com/geovannimp/mixar";
export const QUICK_START_URL = `${GITHUB_REPO}#quick-start`;
export const TECH_SPEC_URL = `${GITHUB_REPO}/blob/main/docs/tech-spec.md`;
export const README_URL = `${GITHUB_REPO}#readme`;
export const CONTRIBUTING_URL = `${GITHUB_REPO}/contribute`;

/** Prefix a site path with Astro `base` (needed on GitHub Pages). */
export function withBase(path = "/"): string {
  const base = import.meta.env.BASE_URL;
  if (path.startsWith("#")) return path;
  if (path.startsWith("/#")) return `${base}${path.slice(1)}`;
  const rel = path.replace(/^\//, "");
  return rel ? `${base}${rel}` : base;
}
