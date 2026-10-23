import { describe, expect, it } from "vitest";

import { createDocsFuseSearch } from "./docs-fuse-search";
import type { SearchDocument } from "./docs-search";

function document(
  overrides: Partial<SearchDocument> & Pick<SearchDocument, "title" | "path" | "category">,
): SearchDocument {
  return {
    description: "",
    topic: "",
    text: "",
    ...overrides,
  };
}

const documents = [
  document({
    title: "MIDI mappings",
    path: "/docs/developers/midi-mappings",
    category: "Developers",
    topic: "MIDI mappings",
    text: "Configure MIDI controller mappings.",
  }),
  document({
    title: "Performance pads",
    path: "/docs/users/performance-pads",
    category: "Users",
    topic: "Performance pads",
    text: "Play cues, loops, and stems with the pads.",
  }),
];

const search = createDocsFuseSearch(documents);

describe("Fuse docs search", () => {
  it("tolerates a typo in multi-token queries", () => {
    expect(search("midi maping", "developers").map((result) => result.document.title)).toEqual([
      "MIDI mappings",
    ]);
  });

  it("limits results to the selected audience", () => {
    expect(search("pads", "developers")).toEqual([]);
    expect(search("midi", "users")).toEqual([]);
    expect(search("midi", "developers").map((result) => result.document.category)).toEqual([
      "Developers",
    ]);
  });
  it("applies the per-page cap while filling the requested result limit", () => {
    const repeatedSections = Array.from({ length: 4 }, (_, index) =>
      document({
        title: "Pads",
        section: `Section ${index}`,
        path: "/docs/users/pads",
        category: "Users",
        topic: "Pads",
        text: "Performance pads.",
      }),
    );
    const otherPage = document({
      title: "Pads keyboard",
      path: "/docs/users/keyboard",
      category: "Users",
      topic: "Keyboard",
      text: "Performance pads.",
    });
    const scopedSearch = createDocsFuseSearch([...repeatedSections, otherPage]);

    const results = scopedSearch("pads", "users", 3);

    expect(results).toHaveLength(3);
    expect(results.filter((result) => result.document.path === "/docs/users/pads")).toHaveLength(2);
    expect(results[2].document.path).toBe("/docs/users/keyboard");
  });
});
