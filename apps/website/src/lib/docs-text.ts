import GithubSlugger from "github-slugger";

/**
 * Plain text and headings for the offline docs search index.
 *
 * The index is built from the authored MDX rather than from rendered HTML, so
 * this module owns the two risky conversions: splitting a page at its `##`
 * headings (and reproducing the heading ids Astro emits), and flattening
 * markdown/MDX into indexable prose. Everything here is pure so it can be unit
 * tested without an Astro build.
 */

/** A page's text before its first `##`, then one chunk per `##` heading. */
export interface DocSection {
  /** Plain-text heading. Absent for the text before the first `##`. */
  heading?: string;
  /** Heading id Astro emits (`github-slugger`, one slugger per page). */
  anchor?: string;
  markdown: string;
}

const FENCE_OPEN = /^ {0,3}(`{3,}|~{3,})/;
const FENCE_CLOSE = /^ {0,3}(`{3,}|~{3,})[ \t]*$/;
const HEADING = /^ {0,3}(#{1,6})[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$/;
const TABLE_RULE = /^ {0,3}\|?[\s:|-]+\|?[ \t]*$/;

function scanFence(
  line: string,
  fence: string | undefined,
): { fenced: boolean; content: boolean; next?: string } {
  if (fence !== undefined) {
    const close = FENCE_CLOSE.exec(line);
    if (close !== null && close[1][0] === fence[0] && close[1].length >= fence.length) {
      return { fenced: true, content: false };
    }
    return { fenced: true, content: true, next: fence };
  }

  const open = FENCE_OPEN.exec(line);
  return open === null
    ? { fenced: false, content: false }
    : { fenced: true, content: false, next: open[1] };
}

/**
 * Split a page into intro + `##` sections. `###` and deeper stay inside their
 * enclosing section (they remain searchable but do not become results), and
 * every heading still advances the slugger so duplicate ids match Astro's.
 */
export function splitIntoSections(body: string): DocSection[] {
  const slugger = new GithubSlugger();
  const sections: DocSection[] = [];
  const lines: string[] = [];
  let current: DocSection = { markdown: "" };
  let fence: string | undefined;

  const flush = () => {
    current.markdown = lines.join("\n");
    sections.push(current);
    lines.length = 0;
  };

  for (const line of body.split(/\r?\n/)) {
    const scannedFence = scanFence(line, fence);
    fence = scannedFence.next;
    if (scannedFence.fenced) {
      if (scannedFence.content) {
        // Code stays searchable (aliases, keys, config names) but is not prose.
        lines.push(line);
      }
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading === null) {
      lines.push(line);
      continue;
    }

    const text = markdownToText(heading[2]).trim();
    const anchor = slugger.slug(text);
    if (heading[1].length !== 2) {
      // Lose the markers, keep the words: `### Foo` reads as `Foo`.
      lines.push(text);
      continue;
    }

    flush();
    current = { heading: text, anchor, markdown: "" };
  }

  flush();
  // A page that opens on `##` has no intro to index.
  return sections.filter((section) => section.heading !== undefined || section.markdown.trim() !== "");
}

/**
 * Flatten markdown/MDX into one line of text. Fenced code keeps its contents
 * (search should find `cc14` and `map.toml`); tables lose their pipes; JSX tags
 * and MDX expressions are dropped rather than indexed as markup.
 */
export function markdownToText(markdown: string): string {
  const out: string[] = [];
  let fence: string | undefined;

  for (let line of markdown.split(/\r?\n/)) {
    const scannedFence = scanFence(line, fence);
    fence = scannedFence.next;
    if (scannedFence.fenced) {
      if (scannedFence.content) out.push(line);
      continue;
    }

    if (line.includes("-") && TABLE_RULE.test(line)) continue;

    const text = inlineToText(
      line
        .replace(/^\s{0,3}>\s?/, "")
        .replace(/^\s*(?:[-*+]|\d+[.)])\s+/, "")
        .replace(/^ {0,3}#{1,6}\s+/, ""),
    );
    if (text !== "") out.push(text);
  }

  return out.join(" ").replace(/\s+/g, " ").trim();
}

const ENTITIES: Record<string, string> = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
};

function decodeEntity(entity: string, code: string): string {
  const named = ENTITIES[code.toLowerCase()];
  if (named !== undefined) return named;
  let point = Number.NaN;
  if (/^#\d+$/.test(code)) point = Number(code.slice(1));
  else if (/^#x[0-9a-f]+$/i.test(code)) point = Number.parseInt(code.slice(2), 16);
  return Number.isInteger(point) && point >= 0 && point <= 0x10ffff
    ? String.fromCodePoint(point)
    : entity;
}

function inlineToText(line: string): string {
  // Inline code is literal text: protect it from tag/expression stripping
  // first, then restore it. Otherwise `<inputs>` or `{app_support}` vanish.
  const code: string[] = [];
  let text = line.replace(/`+([^`]*)`+/g, (_, contents: string) => {
    code.push(contents);
    return `\u0000${code.length - 1}\u0000`;
  });

  text = text
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/<([A-Za-z][\w.-]*)(?:\s[^<>]*?)?\/>/g, " ")
    .replace(/<([A-Za-z][\w.-]*)(?:\s[^<>]*?)?>/g, " ")
    .replace(/<\/([A-Za-z][\w.-]*)\s*>/g, " ")
    .replace(/\{[^}]*\}/g, " ")
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/!?\[([^\]]*)\]\[[^\]]*\]/g, "$1")
    .replace(/~~([^~]+)~~/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/\*([^*\n]+)\*/g, "$1")
    .replace(/(^|[^\w])_([^_\n]+)_/g, "$1$2")
    .replace(/\\[\\`*_{}[\]()#+\-.!|]/g, (escape) => escape[1])
    .replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, decodeEntity)
    .replace(/\|/g, " ");

  return text
    .replace(/\u0000(\d+)\u0000/g, (_, index: string) => code[Number(index)] ?? "")
    .replace(/\s+/g, " ")
    .trim();
}
