/// <reference types="astro/client" />

/**
 * `astro/client` types `import.meta.env` through Vite's `ImportMetaEnv`, which
 * carries a `[key: string]: any` index signature: an undeclared `PUBLIC_*` key
 * is `any`, so `import.meta.env.PUBLIC_Posthog_KEY` is silently `undefined` at
 * runtime. The members below narrow the three vars `.env.example` documents
 * from `any` to `string | undefined`, so using one as a string is checked.
 *
 * The reference above also supplies Vite/Astro's `import.meta` types on their
 * own, instead of relying on the generated (gitignored) `.astro/types.d.ts`.
 */
interface ImportMetaEnv {
 /** PostHog project token. Unset disables analytics entirely. */
 readonly PUBLIC_POSTHOG_KEY?: string;
 /** Ingestion host; falls back to the EU default when unset. */
 readonly PUBLIC_POSTHOG_HOST?: string;
 /** `"true"` makes the SDK narrate itself in the console. */
 readonly PUBLIC_POSTHOG_DEBUG?: string;
}

/**
 * `.astro` modules are typed by the Astro language server / `astro check`, which
 * `tsc --noEmit` does not run, so a `.ts` file importing one has no type at all.
 * The container-based render tests (`scripts/font-preloads.test.ts`) do exactly
 * that; this gives them Astro's own component type instead of `any`.
 */
declare module "*.astro" {
 import type { AstroComponentFactory } from "astro/runtime/server/index.js";
 const component: AstroComponentFactory;
 export default component;
}
