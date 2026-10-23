import { markdownToText, splitIntoSections } from "./docs-text";
import type { SearchDocument } from "./docs-search";

/**
 * One docs page resolved to the fields the search index needs. Routes, labels
 * and collection entries are resolved by `docs-index.ts`; this module stays
 * free of `astro:content` so its composition can be unit tested.
 */
export interface IndexablePage {
  title: string;
  description: string;
  path: string;
  category: string;
  topic: string;
  body: string;
}

/** One section per page intro and per `##` heading; empty pages still index. */
export function buildDocuments(pages: IndexablePage[]): SearchDocument[] {
  const documents: SearchDocument[] = [];

  for (const page of pages) {
    const { body, ...common } = page;
    const sections = splitIntoSections(body);

    // A frontmatter-only page has no prose and no sections, but searching its
    // title should still find it.
    if (sections.length === 0) {
      documents.push({ ...common, text: "" });
      continue;
    }

    for (const section of sections) {
      documents.push({
        ...common,
        ...(section.heading === undefined ? {} : { section: section.heading }),
        ...(section.anchor === undefined ? {} : { anchor: section.anchor }),
        text: markdownToText(section.markdown),
      });
    }
  }

  return documents;
}
