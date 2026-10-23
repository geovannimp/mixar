import Fuse from "fuse.js";

import { meaningfulTerms, snippetFor, type SearchDocument, type SearchResult } from "./docs-search";

export type SearchScope = "users" | "developers";

const options = {
  keys: [
    { name: "title", weight: 4 },
    { name: "section", weight: 3 },
    { name: "topic", weight: 2 },
    { name: "description", weight: 1.5 },
    { name: "text", weight: 1 },
  ],
  includeScore: true,
  ignoreLocation: true,
  threshold: 0.35,
  useTokenSearch: true,
  tokenMatch: "all" as const,
};

export function createDocsFuseSearch(documents: SearchDocument[]) {
  const collections: Record<SearchScope, SearchDocument[]> = {
    users: documents.filter((document) => document.category === "Users"),
    developers: documents.filter((document) => document.category === "Developers"),
  };
  const indexes = Object.fromEntries(
    Object.entries(collections).map(([scope, collection]) => [scope, new Fuse(collection, options)]),
  ) as Record<SearchScope, Fuse<SearchDocument>>;

  return (query: string, scope: SearchScope, limit = 12): SearchResult[] => {
    const terms = meaningfulTerms(query);
    if (terms.length === 0) return [];

    const results: SearchResult[] = [];
    const pageCount = new Map<string, number>();
    for (const hit of indexes[scope].search(terms.join(" "))) {
      const document = hit.item;
      const count = pageCount.get(document.path) ?? 0;
      if (count >= 2) continue;
      pageCount.set(document.path, count + 1);
      results.push({
        document,
        score: hit.score ?? 1,
        snippet: snippetFor(document.text, terms),
      });
      if (results.length >= limit) break;
    }
    return results;
  };
}
