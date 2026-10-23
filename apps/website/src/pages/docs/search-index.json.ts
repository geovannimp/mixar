import type { APIRoute } from "astro";
import { buildSearchIndex } from "../../lib/docs-index";

/**
 * Static search index for the docs UI (`DocsSearch.astro`). Prerendered at
 * build time, so search is a same-origin JSON fetch with no server behind it.
 */
export const prerender = true;

export const GET: APIRoute = async () => {
  const index = await buildSearchIndex();
  return new Response(JSON.stringify(index), {
    headers: { "content-type": "application/json; charset=utf-8" },
  });
};
