import { describe, expect, it } from "vitest";

import { markdownToText, splitIntoSections } from "./docs-text";

describe("splitIntoSections", () => {
  it("splits at h2 with the heading ids Astro emits", () => {
    const sections = splitIntoSections(
      [
        "Intro text.",
        "",
        "## Binding forms",
        "",
        "Body.",
        "",
        "## `[inputs]` binding forms",
        "",
        "More.",
      ].join("\n"),
    );

    expect(sections.map((section) => section.heading)).toEqual([
      undefined,
      "Binding forms",
      "[inputs] binding forms",
    ]);
    expect(sections.map((section) => section.anchor)).toEqual([
      undefined,
      "binding-forms",
      "inputs-binding-forms",
    ]);
    expect(sections[1].markdown).toContain("Body.");
  });

  it("keeps h3 headings inside their h2 section but searchable", () => {
    const sections = splitIntoSections("## Root\n\n### Nested\n\nNested body.\n");

    expect(sections).toHaveLength(1);
    expect(sections[0].markdown).toContain("Nested");
    expect(sections[0].markdown).toContain("Nested body.");
  });

  it("ignores headings inside fenced code", () => {
    const sections = splitIntoSections("## Real\n\n```toml\n## not a heading\n```\n");

    expect(sections.map((section) => section.anchor)).toEqual(["real"]);
    expect(sections[0].markdown).not.toContain("```");
    expect(sections[0].markdown).toContain("## not a heading");
  });

  it("suffixes repeated headings like GitHub", () => {
    const sections = splitIntoSections("## Same\n\na\n\n## Same\n\nb\n");

    expect(sections.map((section) => section.anchor)).toEqual(["same", "same-1"]);
  });
});

describe("markdownToText", () => {
  it("unwraps links, emphasis, and inline code", () => {
    expect(
      markdownToText("The **library** is [Mixar's](/docs/users/library) `library.db` file."),
    ).toBe("The library is Mixar's library.db file.");
  });

  it("keeps brace text that lives in inline code", () => {
    expect(markdownToText("Models live under `{app_support}/models/`.")).toBe(
      "Models live under {app_support}/models/.",
    );
  });

  it("drops JSX wrappers and their attributes", () => {
    expect(
      markdownToText('<Callout type="warning" title="No [inputs.custom]">\nRejected.\n</Callout>'),
    ).toBe("Rejected.");
  });

  it("keeps fenced code contents", () => {
    expect(markdownToText("```toml\ncc14 = true\n```")).toBe("cc14 = true");
  });

  it("turns table rows into text and drops the rule row", () => {
    expect(markdownToText("| Field | Type |\n| --- | --- |\n| `action` | string |")).toBe(
      "Field Type action string",
    );
  });

  it("decodes entities and ignores invalid code points", () => {
    expect(markdownToText("A &amp; B &#8212; C &#999999999;")).toBe("A & B — C &#999999999;");
  });
});
