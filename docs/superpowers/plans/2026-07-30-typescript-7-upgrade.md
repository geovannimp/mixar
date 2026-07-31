# TypeScript 7 Upgrade (via 6) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade `apps/gui-app` from TypeScript `~5.8.3` to `~7.0.2` in one PR with two commits (6 then 7), modernizing tsconfig on the 6 commit.

**Architecture:** Commit 1 installs TypeScript 6, modernizes `tsconfig.json` / `tsconfig.node.json`, adds `@types/node`, and clears Node `@ts-expect-error` hacks. Commit 2 only bumps to TypeScript 7 and re-verifies. No oxlint type-aware / lefthook changes.

**Tech Stack:** TypeScript 6.0.3 → 7.0.2, Vite 7, Vitest, npm workspaces (`apps/gui-app`).

**Spec:** `docs/superpowers/specs/2026-07-30-typescript-7-upgrade-design.md`

## Global Constraints

- One PR, two commits: TS 6 + config first, then TS 7 bump.
- Do not add `oxlint-tsgolint`, `typeAware`, `typeCheck`, or lefthook changes.
- Do not set `ignoreDeprecations`.
- Prefer `npm install` from repo root; verify with `npx tsc --noEmit`, `npm run build`, `npm test` in `apps/gui-app`.
- Pin versions: `typescript@~6.0.3` then `~7.0.2`; add `@types/node` as needed for `"types": ["node"]`.

## File map

| File | Role |
|------|------|
| `apps/gui-app/package.json` | Bump `typescript`; add `@types/node` |
| `package-lock.json` | Lockfile updates |
| `apps/gui-app/tsconfig.json` | `target`/`lib`/`rootDir` modernization |
| `apps/gui-app/tsconfig.node.json` | `"types": ["node"]` |
| `apps/gui-app/vite.config.ts` | Remove obsolete `@ts-expect-error` for `process` |
| `docs/superpowers/specs/2026-07-30-typescript-7-upgrade-design.md` | Spec (already written) |
| `docs/superpowers/plans/2026-07-30-typescript-7-upgrade.md` | This plan |

---

### Task 1: TypeScript 6 + modernized tsconfig

**Files:**
- Modify: `apps/gui-app/package.json`
- Modify: `package-lock.json` (via `npm install`)
- Modify: `apps/gui-app/tsconfig.json`
- Modify: `apps/gui-app/tsconfig.node.json`
- Modify: `apps/gui-app/vite.config.ts`
- Create (if not already on branch): `docs/superpowers/specs/2026-07-30-typescript-7-upgrade-design.md`
- Create (if not already on branch): `docs/superpowers/plans/2026-07-30-typescript-7-upgrade.md`

**Interfaces:**
- Produces: gui-app on `typescript@~6.0.3` with ES2025 tsconfig and Node types for Vite config

- [ ] **Step 1: Branch from main**

```bash
git fetch origin main
git checkout -b chore/typescript-7-upgrade origin/main
```

Expected: clean branch based on latest `main`.

- [ ] **Step 2: Bump package deps**

In `apps/gui-app/package.json` `devDependencies`:

```json
"@types/node": "^26.0.0",
"typescript": "~6.0.3",
```

Run from repo root:

```bash
npm install
```

Expected: lockfile updates; `typescript@6.0.3` (or latest 6.0.x matching `~6.0.3`) installed for gui-app.

- [ ] **Step 3: Modernize `tsconfig.json`**

Replace compiler `target` / `lib` and add `rootDir`:

```json
{
  "compilerOptions": {
    "target": "ES2025",
    "useDefineForClassFields": true,
    "lib": ["ES2025", "DOM"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "rootDir": "./src",
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

Do not add `ignoreDeprecations`.

- [ ] **Step 4: Modernize `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "types": ["node"]
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Clean `vite.config.ts`**

Remove the `@ts-expect-error` above `process` so it becomes:

```ts
const host = process.env.TAURI_DEV_HOST;
```

If `tsc -p tsconfig.node.json` still fails on `process`, keep investigating `@types/node` install before re-adding expect-error.

- [ ] **Step 6: Verify on TypeScript 6**

```bash
cd apps/gui-app
npx tsc --noEmit
npx tsc -p tsconfig.node.json --noEmit
npm run build
npm test
```

Expected: all PASS. If source errors appear from ES2025/`types` defaults, fix them in this commit (minimal diffs).

- [ ] **Step 7: Commit**

```bash
git add apps/gui-app/package.json package-lock.json \
  apps/gui-app/tsconfig.json apps/gui-app/tsconfig.node.json \
  apps/gui-app/vite.config.ts \
  docs/superpowers/specs/2026-07-30-typescript-7-upgrade-design.md \
  docs/superpowers/plans/2026-07-30-typescript-7-upgrade.md
git commit -m "$(cat <<'EOF'
chore(gui-app): upgrade TypeScript to 6 with modernized tsconfig

EOF
)"
```

---

### Task 2: TypeScript 7 bump

**Files:**
- Modify: `apps/gui-app/package.json`
- Modify: `package-lock.json`

**Interfaces:**
- Consumes: Task 1 tsconfigs with no deprecated options
- Produces: gui-app on `typescript@~7.0.2`

- [ ] **Step 1: Bump typescript**

In `apps/gui-app/package.json`:

```json
"typescript": "~7.0.2",
```

```bash
npm install
```

Expected: `typescript@7.0.2` (or latest matching `~7.0.2`).

- [ ] **Step 2: Verify on TypeScript 7**

```bash
cd apps/gui-app
npx tsc --noEmit
npx tsc -p tsconfig.node.json --noEmit
npm run build
npm test
```

Expected: all PASS. Fix only residual breaks required by 7.

- [ ] **Step 3: Commit**

```bash
git add apps/gui-app/package.json package-lock.json
git commit -m "$(cat <<'EOF'
chore(gui-app): upgrade TypeScript to 7

EOF
)"
```

---

### Task 3: Open PR and note #99

**Files:** none (git / gh only)

- [ ] **Step 1: Push and create PR**

```bash
git push -u origin HEAD
gh pr create --title "chore(gui-app): upgrade TypeScript to 7 via 6" --body "$(cat <<'EOF'
## Summary
- Upgrade `apps/gui-app` TypeScript `~5.8.3` → `~6.0.3` with modernized tsconfig (`target`/`lib` ES2025, `rootDir`, Node `types` for Vite config), then → `~7.0.2`.
- Adds `@types/node` so `tsconfig.node.json` works under 6/7 `types` defaults; removes obsolete `process` `@ts-expect-error`.
- Deliberately defers `oxlint-tsgolint` / type-aware `typeCheck` and lefthook wiring to #99.

Closes #115.

## Test plan
- [ ] `cd apps/gui-app && npx tsc --noEmit`
- [ ] `cd apps/gui-app && npx tsc -p tsconfig.node.json --noEmit`
- [ ] `cd apps/gui-app && npm run build`
- [ ] `cd apps/gui-app && npm test`

EOF
)"
```

- [ ] **Step 2: Comment on #99**

```bash
gh issue comment 99 --repo geovannimp/rust-dj-engine --body "$(cat <<'EOF'
TypeScript upgrade for gui-app landed in the PR for #115 (TS 7 via 6). Ready for the preferred oxlint `--type-aware --type-check` path when wiring pre-commit typecheck; `oxlint-tsgolint` was intentionally not added in #115.

EOF
)"
```

Expected: PR URL returned; comment visible on #99.
