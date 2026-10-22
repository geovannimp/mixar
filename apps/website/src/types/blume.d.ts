// The virtual modules Blume's generated runtime exposes to the site's own
// pages. `blume check` would pick these up from `.blume/.astro`, but the
// website typechecks with `tsc` (TypeScript 7's native compiler has no
// programmatic API for `astro check`), so the site's own declarations stand
// in for them.
//
// Only the modules the site actually imports are declared here — an unused
// declaration would still pull Blume's own sources into the typecheck.
declare module "blume:data" {
  const data: import("blume").BlumeData;
  export default data;
}
