import { categorySlug, docPath, getDocs, topicSlug } from "./docs";
import { buildDocuments, type IndexablePage } from "./docs-index-core";
import type { SearchIndex } from "./docs-search";

/**
 * Build the offline search index from the docs collection.
 *
 * Runs at build time (see `src/pages/docs/search-index.json.ts`), so the
 * shipped JSON is whatever the site was built from — no separate content
 * pipeline, and a docs typo can never drift out of the index.
 */
export async function buildSearchIndex(): Promise<SearchIndex> {
  const { docs, categories } = await getDocs();

  const pages: IndexablePage[] = docs.map((entry) => {
    const category = categories.find((candidate) => candidate.slug === categorySlug(entry.id));
    const topic = category?.topics.find((candidate) => candidate.slug === topicSlug(entry.id));
    return {
      title: entry.data.title,
      description: entry.data.description,
      path: docPath(entry.id),
      category: category?.name ?? entry.data.section,
      topic: topic?.name ?? entry.data.section,
      body: entry.body ?? "",
    };
  });

  return { documents: buildDocuments(pages) };
}
