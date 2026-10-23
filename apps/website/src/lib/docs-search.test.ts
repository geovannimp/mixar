import { describe, expect, it } from "vitest";

import { meaningfulTerms, snippetFor, tokenize } from "./docs-search";

function highlightedParts(snippet: ReturnType<typeof snippetFor>): string[] {
  return snippet.filter((part) => part.match).map((part) => part.text);
}

describe("meaningfulTerms", () => {
  it("drops stop words while retaining search terms", () => {
    expect(meaningfulTerms("how do I configure the pads")).toEqual(["configure", "pads"]);
  });
});

describe("tokenize", () => {
  it("lowercases, strips diacritics, and keeps digits", () => {
    expect(tokenize("Café CC14 — [inputs]")).toEqual(["cafe", "cc14", "inputs"]);
  });
});


describe("snippetFor", () => {
  it("windows around the match and highlights the whole token", () => {
    const text = `${"padding ".repeat(30)}separation runs offline${" tail".repeat(30)}`;
    const snippet = snippetFor(text, ["separ"]);

    expect(highlightedParts(snippet)).toEqual(["separation"]);

    const plain = snippet.map((part) => part.text).join("");
    expect(plain).toContain("runs offline");
    expect(plain.startsWith("…")).toBe(true);
    expect(plain.endsWith("…")).toBe(true);
  });

  it("highlights a token matched only through the singular/plural rule", () => {
    const snippet = snippetFor("Stem files live under app support.", ["stems"]);

    expect(highlightedParts(snippet)).toEqual(["Stem"]);
  });

  it("highlights accented tokens matched after diacritic stripping", () => {
    const snippet = snippetFor("A Café set.", ["cafe"]);

    expect(highlightedParts(snippet)).toEqual(["Café"]);
  });

  it("falls back to the start when the term is only in metadata", () => {
    const snippet = snippetFor("Short body.", ["title-only"]);

    expect(snippet).toEqual([{ text: "Short body.", match: false }]);
  });

  it("merges overlapping matches", () => {
    const snippet = snippetFor("library", ["lib", "library"]);

    expect(highlightedParts(snippet)).toEqual(["library"]);
  });

  it("highlights typo-tolerant matches", () => {
    const snippet = snippetFor("MIDI mappings", ["midi", "maping"]);

    expect(highlightedParts(snippet)).toEqual(["MIDI", "mappings"]);
  });
});

