/** Shared document, result, and highlighting types for the static docs search. */
export interface SearchDocument {
  /** Page title from frontmatter. */
  title: string;
  /** `##` heading text, absent for a page's intro section. */
  section?: string;
  /** Heading id, for `#anchor` links. */
  anchor?: string;
  /** Canonical docs route (`/docs/<category>/<topic>/[...]`). */
  path: string;
  /** Display name of the audience (`Users`, `Developers`). */
  category: string;
  /** Display name of the topic (`MIDI Mappings`). */
  topic: string;
  /** Page description from frontmatter. */
  description: string;
  /** Flattened section prose. */
  text: string;
}

export interface SearchIndex {
  documents: SearchDocument[];
}

export interface SearchSnippet {
  text: string;
  match: boolean;
}

export interface SearchResult {
  document: SearchDocument;
  score: number;
  snippet: SearchSnippet[];
}

const WORD = /[\p{L}\p{N}]+/gu;

/**
 * Query filler that only creates false negatives under AND matching: a page
 * about "the library" matches `library`, not `library how do i`.
 */
const STOP_WORDS: Record<string, true> = {
  a: true,
  an: true,
  and: true,
  are: true,
  as: true,
  at: true,
  be: true,
  but: true,
  by: true,
  can: true,
  do: true,
  for: true,
  from: true,
  has: true,
  have: true,
  how: true,
  i: true,
  if: true,
  in: true,
  is: true,
  it: true,
  not: true,
  of: true,
  on: true,
  or: true,
  so: true,
  the: true,
  to: true,
  was: true,
  we: true,
  what: true,
  with: true,
};

/** Lowercase, strip diacritics, split on anything that is not a letter/digit. */
export function tokenize(text: string): string[] {
  return (
    text
      .toLowerCase()
      .normalize("NFKD")
      .replace(/\p{M}+/gu, "")
      .match(WORD) ?? []
  );
}

/** The query's terms with the filler words dropped. */
export function meaningfulTerms(query: string): string[] {
  return tokenize(query).filter((term) => !Object.hasOwn(STOP_WORDS, term));
}



/** The prefix and edit-distance rules used to highlight Fuse matches. */
function matchesTerm(token: string, term: string): boolean {
  const isStop = Object.hasOwn(STOP_WORDS, token);
  if (
    token === term ||
    token.startsWith(term) ||
    (!isStop && term.length >= 4 && token.length >= 3 && term.startsWith(token))
  ) {
    return true;
  }

  const maxDistance = Math.floor(Math.max(token.length, term.length) * 0.35);
  if (maxDistance === 0 || Math.abs(token.length - term.length) > maxDistance) return false;

  let previous = Array.from({ length: term.length + 1 }, (_, index) => index);
  for (let tokenIndex = 1; tokenIndex <= token.length; tokenIndex++) {
    const current = [tokenIndex];
    let rowMinimum = tokenIndex;
    for (let termIndex = 1; termIndex <= term.length; termIndex++) {
      const distance = Math.min(
        current[termIndex - 1] + 1,
        previous[termIndex] + 1,
        previous[termIndex - 1] + Number(token[tokenIndex - 1] !== term[termIndex - 1]),
      );
      current.push(distance);
      rowMinimum = Math.min(rowMinimum, distance);
    }
    if (rowMinimum > maxDistance) return false;
    previous = current;
  }
  return previous[term.length] <= maxDistance;
}


const SNIPPET_RADIUS = 100;

/**
 * A window of `text` around the first match, split into highlighted runs.
 * Extracts a character window from the original text (preserving spaces,
 * punctuation, and casing), then highlights matching tokens within it.
 * Adds "…" prefix/suffix when the window is truncated.
 */
export function snippetFor(text: string, terms: string[], radius = SNIPPET_RADIUS): SearchSnippet[] {
  if (terms.length === 0) return [];

  // Search in diacritic-stripped lowercase text for case/diacritic-insensitive matching.
  const searchText = text
    .toLowerCase()
    .normalize("NFKD")
    .replace(/\p{M}+/gu, "");

  // Find the first token position that matches any term.
  const tokens = tokenize(text);
  let matchPos = -1;

  for (const token of tokens) {
    const pos = searchText.indexOf(token);
    if (pos === -1) continue;

    for (const term of terms) {
      if (matchesTerm(token, term)) {
        matchPos = pos;
        break;
      }
    }
    if (matchPos !== -1) break;
  }

  if (matchPos === -1) {
    // No match in this text (the hit was in title/section/description).
    // Return the start of the text as an unhighlighted snippet.
    return [{ text: text.slice(0, radius), match: false }];
  }

  // Extract a character window around the match from the ORIGINAL text.
  // For ASCII text, searchText positions align with original text positions.
  // For diacritic text, this is an approximation but close enough for snippets.
  const start = Math.max(0, matchPos - radius);
  const end = Math.min(text.length, matchPos + radius);
  const windowText = text.slice(start, end);

  const truncatedStart = start > 0;
  const truncatedEnd = end < text.length;

  // Highlight matching tokens within the window, preserving all text between tokens.
  const windowTokens = tokenize(windowText);
  const windowSearchText = windowText
    .toLowerCase()
    .normalize("NFKD")
    .replace(/\p{M}+/gu, "");

  const snippet: SearchSnippet[] = [];
  let offset = 0;
  let lastEnd = 0;

  for (const token of windowTokens) {
    const tokenStart = windowSearchText.indexOf(token, offset);
    if (tokenStart === -1) break;
    const tokenEnd = tokenStart + token.length;

    // Include the text between the last token and this one (spaces, punctuation).
    if (tokenStart > lastEnd) {
      snippet.push({
        text: windowText.slice(lastEnd, tokenStart),
        match: false,
      });
    }

    let match = false;
    for (const term of terms) {
      if (matchesTerm(token, term)) {
        match = true;
        break;
      }
    }

    snippet.push({
      text: windowText.slice(tokenStart, tokenEnd),
      match,
    });

    offset = tokenEnd;
    lastEnd = tokenEnd;
  }

  // Any trailing text after the last token.
  if (lastEnd < windowText.length) {
    snippet.push({
      text: windowText.slice(lastEnd),
      match: false,
    });
  }

  // Add ellipsis to first/last parts when window is truncated.
  if (snippet.length > 0) {
    if (truncatedStart) {
      snippet[0].text = "…" + snippet[0].text;
    }
    if (truncatedEnd) {
      const last = snippet.length - 1;
      snippet[last].text = snippet[last].text + "…";
    }
  }

  return snippet;
}