import { describe, expect, it } from "vitest";

import { buildDocuments, type IndexablePage } from "./docs-index-core";

function page(overrides: Partial<IndexablePage> = {}): IndexablePage {
  return {
    title: "Library",
    description: "What the library holds.",
    path: "/docs/users/library",
    category: "Users",
    topic: "Library",
    body: "",
    ...overrides,
  };
}

describe("buildDocuments", () => {
  it("splits a page into an intro document and one per h2", () => {
    const documents = buildDocuments([
      page({
        body: [
          "Intro text.",
          "",
          "## Getting music in",
          "",
          "Body text.",
          "",
          "### Nested detail",
          "",
          "More text.",
        ].join("\n"),
      }),
    ]);

    expect(documents).toHaveLength(2);
    expect(documents[0]).toMatchObject({ text: "Intro text." });
    expect(documents[0].section).toBeUndefined();
    expect(documents[0].anchor).toBeUndefined();
    expect(documents[1]).toMatchObject({
      section: "Getting music in",
      anchor: "getting-music-in",
      text: "Body text. Nested detail More text.",
    });
    // Resolved metadata travels with every section.
    expect(documents[1]).toMatchObject({
      title: "Library",
      path: "/docs/users/library",
      category: "Users",
      topic: "Library",
    });
  });

  it("keeps a page with no body searchable by its title", () => {
    const documents = buildDocuments([page({ title: "Empty page" })]);

    expect(documents).toEqual([
      {
        title: "Empty page",
        description: "What the library holds.",
        path: "/docs/users/library",
        category: "Users",
        topic: "Library",
        text: "",
      },
    ]);
  });

  it("does not emit an empty intro for a page that opens on a heading", () => {
    const documents = buildDocuments([page({ body: "## Only section\n\nText." })]);

    expect(documents).toHaveLength(1);
    expect(documents[0]).toMatchObject({ section: "Only section", anchor: "only-section" });
  });
});
