import { getCollection, type CollectionEntry } from "astro:content";

export type DocEntry = CollectionEntry<"docs">;

export interface DocTopic {
  slug: string;
  name: string;
  /** The topic's `overview` entry; the topic is served at its path. */
  overview: DocEntry;
  /** Flat, display-sorted — used for prev/next. */
  entries: DocEntry[];
  /** Same entries as a tree: pages in a nested folder hang off their parent. */
  nav: DocNode[];
}

/** A docs page plus the nested pages that live in a folder under it. */
export interface DocNode {
  entry: DocEntry;
  /** Set when the page lives in a folder under another page (`map-toml/inputs`). */
  parent?: DocNode;
  children: DocNode[];
}

export interface DocCategory {
  slug: string;
  name: string;
  description: string;
  /** Shown on `/docs` while the category has no topics yet. */
  empty: string;
  topics: DocTopic[];
}

/**
 * Top-level docs categories, in display order. A category that has no entries
 * yet (e.g. Users) still lists on `/docs` so the structure is visible.
 *
 * The slug must match the first path segment under `src/content/docs/`, or the
 * build fails in `getDocs()`.
 */
export const CATEGORIES = [
  {
    slug: "users",
    name: "Users",
    description: "Using Mixar to play and organize music.",
    empty: "User guides are on the way.",
  },
  {
    slug: "developers",
    name: "Developers",
    description: "Building, extending, and integrating with Mixar.",
    empty: "Developer guides are on the way.",
  },
] as const;

function segments(id: string): string[] {
  return id.split("/");
}

/** First path segment of an entry id: `developers/midi-mappings/overview` → `developers`. */
export function categorySlug(id: string): string {
  return segments(id)[0];
}

/** Second path segment: the topic within the category. */
export function topicSlug(id: string): string {
  return segments(id)[1] ?? "";
}

/**
 * A topic's overview entry lives at `<category>/<topic>/overview` but is served
 * at the topic root (`/docs/<category>/<topic>`); every other entry keeps its id.
 */
export function docSlug(id: string): string {
  return id.endsWith("/overview") ? id.slice(0, -"/overview".length) : id;
}

export function docPath(id: string): string {
  return `/docs/${docSlug(id)}`;
}

export interface Docs {
  docs: DocEntry[];
  categories: DocCategory[];
}

// Loaded and grouped on every route that imports this module (`getStaticPaths`,
// the page, and the layout). In a build, memoizing collapses those into one
// computation; in dev, always reload so content edits are picked up directly.
let cache: Promise<Docs> | undefined;

export function getDocs(): Promise<Docs> {
  if (!import.meta.env.PROD) {
    return loadDocs();
  }
  cache ??= loadDocs();
  return cache;
}

async function loadDocs(): Promise<Docs> {
  const docs = (await getCollection("docs")).sort(
    (a, b) => a.data.order - b.data.order || a.id.localeCompare(b.id),
  );

  const known = new Set<string>(CATEGORIES.map((category) => category.slug));
  for (const entry of docs) {
    if (segments(entry.id).length < 3) {
      throw new Error(
        `docs: entry "${entry.id}" must live at <category>/<topic>/<page>.mdx ` +
          `(nothing directly under a category or a topic folder).`,
      );
    }
    if (!known.has(categorySlug(entry.id))) {
      throw new Error(
        `docs: entry "${entry.id}" is not under a known category ` +
          `(expected one of: ${[...known].join(", ")}). ` +
          `Add the category to CATEGORIES in src/lib/docs.ts.`,
      );
    }
  }

  const categories: DocCategory[] = CATEGORIES.map((category) => {
    const grouped = new Map<string, DocEntry[]>();
    for (const entry of docs) {
      if (categorySlug(entry.id) !== category.slug) continue;
      const slug = topicSlug(entry.id);
      const bucket = grouped.get(slug);
      if (bucket === undefined) {
        grouped.set(slug, [entry]);
      } else {
        bucket.push(entry);
      }
    }

    const topics: DocTopic[] = [...grouped.entries()].map(([slug, entries]) => {
      const overview = entries.find((entry) => entry.id.endsWith("/overview"));
      if (overview === undefined) {
        throw new Error(
          `docs: topic "${category.slug}/${slug}" has no overview entry ` +
            `(expected ${category.slug}/${slug}/overview.mdx).`,
        );
      }
      return { slug, name: overview.data.section, overview, entries, nav: buildDocNav(entries) };
    });

    return {
      slug: category.slug,
      name: category.name,
      description: category.description,
      empty: category.empty,
      topics,
    };
  });

  return { docs, categories };
}

// Top-level pages are `<category>/<topic>/<page>` (3 segments); anything deeper
// lives in a folder under another page. Keep in sync with the loadDocs check
// that rejects entries shorter than `<category>/<topic>/<page>`.
const TOP_LEVEL_SEGMENTS = 3;

/**
 * Folder a nested page lives in: `developers/midi-mappings/map-toml/inputs`
 * → `developers/midi-mappings/map-toml`. `undefined` for top-level pages
 * (`<category>/<topic>/<page>`).
 */
function parentDocPath(id: string): string | undefined {
  const parts = segments(id);
  if (parts.length <= TOP_LEVEL_SEGMENTS) return undefined;
  parts.pop();
  return parts.join("/");
}

/**
 * Group a topic's entries so pages in a nested folder hang off the page whose
 * route is their folder. A nested page whose folder has no page of its own
 * stays top-level rather than disappearing.
 */
function buildDocNav(entries: DocEntry[]): DocNode[] {
  const nodes: DocNode[] = entries.map((entry) => ({ entry, children: [] }));
  // Route path → node so a nested page finds its parent in O(1). Route paths
  // always contain `/`, so they can never collide with an Object.prototype key.
  const byPath: Record<string, DocNode> = {};
  for (const node of nodes) byPath[docSlug(node.entry.id)] = node;
  const roots: DocNode[] = [];
  for (const node of nodes) {
    const parentPath = parentDocPath(node.entry.id);
    const parent = parentPath === undefined ? undefined : byPath[parentPath];
    // `parent === node` is the folder's own `overview` page: `docSlug` strips the
    // trailing `/overview`, so `.../map-toml/overview` hashes to the very route
    // `parentDocPath` returns for it. Without this guard the overview would
    // become its own child.
    if (parent === undefined || parent === node) {
      roots.push(node);
      continue;
    }
    node.parent = parent;
    parent.children.push(node);
  }
  return roots;
}

/**
 * Sidebar label for a node: nested pages drop the parent title prefix, so
 * `map.toml — [inputs]` renders as `[inputs]` under its `map.toml` parent. The
 * prefix only counts when what follows is a separator character (`—`, `-`, or
 * `:`), optionally surrounded by whitespace, and anything that would strip away
 * the whole title keeps the full title instead.
 */
export function navLabel(node: DocNode): string {
  const title = node.entry.data.title;
  const parentTitle = node.parent?.entry.data.title;
  if (parentTitle === undefined || !title.startsWith(parentTitle)) return title;
  const rest = title.slice(parentTitle.length);
  if (!/^\s*[—:-]/.test(rest)) return title;
  return rest.replace(/^\s*[—:-]\s*/, "") || title;
}

/** Entries of the topic an entry belongs to, for prev/next within that topic. */
export function entriesForTopic(categories: DocCategory[], id: string): DocEntry[] {
  const category = categories.find((c) => c.slug === categorySlug(id));
  const topic = category?.topics.find((t) => t.slug === topicSlug(id));
  if (topic === undefined) {
    throw new Error(`docs: no topic found for entry "${id}"`);
  }
  return topic.entries;
}
