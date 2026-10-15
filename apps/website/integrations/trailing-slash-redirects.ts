import { mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import type { AstroIntegration } from "astro";

/**
 * `build.format: "file"` emits `foo.html` per route so GitHub Pages serves the
 * slashless URL directly — but Pages only answers `/foo/` from
 * `foo/index.html`, so every previously canonical trailing-slash URL would
 * 404. This integration writes a redirect stub at each trailing-slash path:
 * a meta refresh plus a canonical link pointing at the slashless URL.
 *
 * Stub writing is best-effort: a failure logs a warning instead of aborting the
 * build, so a dist problem can never fail the whole site build.
 */
export default function trailingSlashRedirects(): AstroIntegration {
  let site: string | undefined;

  return {
    name: "trailing-slash-redirects",
    hooks: {
      "astro:config:done": ({ config }) => {
        site = config.site;
      },
      "astro:build:done": async ({ dir, logger }) => {
        // `dir` is a URL: `.pathname` is percent-encoded and Windows-hostile
        // (`/C:/…`), so go through fileURLToPath for a real filesystem path.
        const outDir = fileURLToPath(dir);
        let pages: string[];
        try {
          pages = await htmlPages(outDir);
        } catch (error) {
          logger.warn(`trailing-slash redirects: could not scan ${outDir} (${error}); skipping`);
          return;
        }
        if (pages.length === 0) {
          logger.warn(`trailing-slash redirects: no pages in ${outDir}; stubs not written`);
        }
        let written = 0;
        for (const page of pages) {
          const route = relative(outDir, page).replace(/\.html$/, "");
          const stub = join(outDir, route, "index.html");
          let hasStub = false;
          try {
            await stat(stub);
            hasStub = true;
          } catch {
            // No page at this route yet — safe to write the stub.
          }
          // Never clobber a real page (e.g. if the format changes back).
          if (hasStub) continue;
          try {
            await mkdir(dirname(stub), { recursive: true });
            await writeFile(stub, stubPage(`/${route}`, site));
            written += 1;
          } catch (error) {
            logger.warn(`trailing-slash redirects: could not write ${stub} (${error})`);
          }
        }
        logger.info(`trailing-slash redirects: ${written} stub(s) beside ${pages.length} page(s)`);
      },
    },
  };
}

async function htmlPages(outDir: string): Promise<string[]> {
  const out: string[] = [];
  const walk = async (current: string): Promise<void> => {
    const entries = await readdir(current, { withFileTypes: true });
    const subdirs: string[] = [];
    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        subdirs.push(path);
        continue;
      }
      // With `format: "file"` every page emits `<route>.html`, so an
      // `index.html` at any depth is either the site root, a stub from an
      // earlier build (re-stubbing it would be a no-op at best), or a real
      // index page that Pages already serves at its trailing-slash path.
      if (!entry.name.endsWith(".html") || entry.name === "index.html" || entry.name === "404.html")
        continue;
      out.push(path);
    }
    await Promise.all(subdirs.map(walk));
  };
  await walk(outDir);
  return out;
}

/** Routes are file-system derived, so escape before interpolating into HTML. */
function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function stubPage(slashless: string, site: string | undefined): string {
  const escaped = escapeHtml(slashless);
  const canonical = escapeHtml(site ? `${site.replace(/\/$/, "")}${slashless}` : slashless);
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=${escaped}">
    <link rel="canonical" href="${canonical}">
    <title>Redirecting…</title>
  </head>
  <body>
    <a href="${escaped}">Redirecting…</a>
  </body>
</html>
`;
}
