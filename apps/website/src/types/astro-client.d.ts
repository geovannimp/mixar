// Astro's client types give `import.meta.env` its shape (`PUBLIC_*` keys, the
// `DEV`/`PROD`/`SSR` flags). The generated `.astro/types.d.ts` normally
// references them, but the website typechecks with `tsc` over its own sources
// rather than the generated runtime, so the reference is declared here.
/// <reference types="astro/client" />
