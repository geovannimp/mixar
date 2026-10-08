import { defineCollection, z } from "astro:content";
import { glob } from "astro/loaders";

/**
 * Website documentation. Entries live under a category folder (`users`,
 * `developers`), then a topic folder, then the page file — for example
 * `developers/midi-mappings/overview`. The `section` frontmatter names the
 * topic ("MIDI Mappings"); `order` sorts pages within a topic.
 *
 * Routes are `/docs/<category>/<topic>/<page>`, with a topic's `overview`
 * served at the topic root `/docs/<category>/<topic>`.
 */
const docs = defineCollection({
  loader: glob({ pattern: "**/*.{md,mdx}", base: "./src/content/docs" }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    section: z.string(),
    order: z.number().default(0),
  }),
});

export const collections = { docs };
