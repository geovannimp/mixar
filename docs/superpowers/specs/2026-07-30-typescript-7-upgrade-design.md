# Upgrade gui-app to TypeScript 7 (via 6)

Date: 2026-07-30  
Issue: [#115](https://github.com/geovannimp/rust-dj-engine/issues/115)  
Parent gap: [#99](https://github.com/geovannimp/rust-dj-engine/issues/99)  
Status: accepted

## Goal

Upgrade `apps/gui-app` from TypeScript `~5.8.3` to TypeScript `7+` in one PR with two commits: first land on `~6.0.3` with modernized tsconfig, then bump to `~7.0.2`. Typecheck and build must stay clean. Leave oxlint type-aware / lefthook wiring to #99.

## Scope

**In**

- `apps/gui-app` `typescript` version + lockfile
- App and Node/Vite tsconfigs modernized for 6.0 defaults (no `ignoreDeprecations`)
- `@types/node` as a gui-app devDependency when enabling `"types": ["node"]` on the Vite config project
- Source fixes required by the new compiler (including removing obsolete `@ts-expect-error` once Node types work)
- PR notes documenting intentional config migrations
- Comment on #99 after the PR is up: TS upgrade done; oxlint `typeCheck` still open

**Out**

- `oxlint-tsgolint`, `options.typeAware` / `typeCheck`, changing `lint` scripts
- Lefthook / moon / Rust clippy changes (#99)
- Replacing `tsc` in `build` with oxlint `--type-check` in this PR
- TypeScript upgrades outside `apps/gui-app`

## Approach

Aggressive tsconfig modernization on the TypeScript 6 commit, then a small TypeScript 7 bump:

1. **Commit 1 — TypeScript 6 + config migration**  
   Install `typescript@~6.0.3`. Update tsconfigs for 6.0 defaults and deprecations. Add `@types/node` if needed. Verify `tsc --noEmit`, `npm run build`, `npm test`.

2. **Commit 2 — TypeScript 7**  
   Bump to `typescript@~7.0.2`. Re-verify the same commands. Fix any residual breaks only.

## tsconfig (commit 1)

### `apps/gui-app/tsconfig.json`

| Option | From | To | Why |
|--------|------|----|-----|
| `target` | `ES2020` | `ES2025` | Align with 6.0 default |
| `lib` | `["ES2020", "DOM", "DOM.Iterable"]` | `["ES2025", "DOM"]` | `DOM.Iterable` is included in `DOM` in 6.0 |
| `rootDir` | (unset) | `"./src"` | 6.0 defaults `rootDir` to the tsconfig directory; pin for `include: ["src"]` |

Keep: `strict`, `module: "ESNext"`, `moduleResolution: "bundler"`, `noEmit`, `paths` (`@/*` → `./src/*`) without `baseUrl`. Do not set `ignoreDeprecations`. Leave app `types` empty / unset (Vite client types via `src/vite-env.d.ts`).

### `apps/gui-app/tsconfig.node.json`

| Option | Change | Why |
|--------|--------|-----|
| `types` | `["node"]` | 6.0 defaults `types` to `[]`; stop relying on auto-discovery / `@ts-expect-error` for `process` |
| `allowSyntheticDefaultImports` | Keep `true` or remove if redundant | Never set `false` (hard error in 7) |

Keep `moduleResolution: "bundler"` unless verification forces `nodenext`.

### Package

- Commit 1: `typescript` → `~6.0.3`; add `@types/node` (current major) as a gui-app `devDependency` if not already direct.
- Commit 2: `typescript` → `~7.0.2`.
- Update root workspace lockfile each commit.

## Verification

After each commit, from `apps/gui-app`:

1. Root `npm install` (workspace lockfile).
2. `npx tsc --noEmit` — clean; no `ignoreDeprecations`.
3. `npm run build` (`tsc && vite build`).
4. `npm test` (Vitest smoke).

## Acceptance

- [ ] gui-app depends on TypeScript 7+
- [ ] Project typechecks and builds cleanly under 7
- [ ] Intentional config migrations documented in the PR
- [ ] Note left on #99 that the TS upgrade is done
- [ ] Explicitly deferred: `oxlint-tsgolint` / type-aware lint / lefthook typecheck wiring
