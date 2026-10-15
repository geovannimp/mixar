import { mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import type { AstroIntegration } from "astro";

/**
 * `build.format: "file"` emits `foo.html` per route so GitHub Pages serves the
 * slashless URL directly — but Pages only answers `/foo/` from
 * `foo/index.html`, so every previously canonical trailing-slash URL would
 * 404. This integration writes a redirect stub at each trailing-slash path:
 * a meta refresh plus a canonical link pointing at the slashless URL.
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
        const outDir = dir.pathname;
        const pages = await htmlPages(outDir);
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
          await mkdir(dirname(stub), { recursive: true });
          await writeFile(stub, stubPage(`/${route}`, site));
          written += 1;
        }
        logger.info(`trailing-slash redirects: ${written} stub(s) beside ${pages.length} page(s)`);
      },
    },
  };
}

/** Every emitted `*.html` except index and 404 pages. */
async function htmlPages(outDir: string): Promise<string[]> {
  const out: string[] = [];
  const walk = async (current: string): Promise<void> => {
    const entries = await readdir(current, { withFileTypes: true });
    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        await walk(path);
        continue;
      }
      if (!entry.name.endsWith(".html") || entry.name === "index.html" || entry.name === "404.html")
        continue;
      out.push(path);
    }
  };
  await walk(outDir);
  return out;
}

function stubPage(slashless: string, site: string | undefined): string {
  const canonical = site ? `${site.replace(/\/$/, "")}${slashless}` : slashless;
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=${slashless}">
    <link rel="canonical" href="${canonical}">
    <title>Redirecting…</title>
  </head>
  <body>
    <a href="${slashless}">Redirecting…</a>
  </body>
</html>
`;
}
