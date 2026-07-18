# gui-app Frontend Lint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Oxlint + Oxfmt to `gui-app`, wire staged-file lefthook jobs, and extend the CI `lint` job so frontend format/lint gate PRs.

**Architecture:** Tooling lives in the `gui-app` workspace package. Lefthook at the repo root runs format then lint-fix on staged `*.{ts,tsx}` with `stage_fixed`. CI’s existing `lint` job installs Node and runs `format:check` + `lint` via npm workspaces. First land reformats/autofixes the whole TS tree so the baseline is clean.

**Tech Stack:** oxlint, oxfmt, lefthook, npm workspaces, GitHub Actions

## Global Constraints

- Toolchain: Oxlint + Oxfmt (not ESLint/Prettier)
- Scripts and config owned by `gui-app`
- Pre-commit: staged `*.{ts,tsx}` only; `stage_fixed: true`
- CI: extend existing `lint` job only (no separate frontend job yet)
- No full `tsc` in hook or lint job
- Issue: #76

---

### Task 1: Branch + install Oxlint/Oxfmt + scripts/config

**Files:**
- Create: `gui-app/.oxlintrc.json`
- Create: `gui-app/.oxfmtrc.json`
- Modify: `gui-app/package.json`
- Modify: `package-lock.json` (via npm)

**Interfaces:**
- Produces scripts: `lint`, `lint:fix`, `format`, `format:check` on workspace `gui-app`

- [ ] **Step 1: Create branch from `main`**

```bash
git checkout main
git checkout -b feature/issue-76-gui-app-lint
```

- [ ] **Step 2: Install packages in gui-app workspace**

```bash
npm install -D oxlint oxfmt -w gui-app
```

Expected: `gui-app/package.json` gains `oxlint` and `oxfmt` under `devDependencies`; lockfile updates.

- [ ] **Step 3: Add scripts to `gui-app/package.json`**

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "tauri": "tauri",
  "lint": "oxlint .",
  "lint:fix": "oxlint --fix .",
  "format": "oxfmt .",
  "format:check": "oxfmt --check ."
}
```

- [ ] **Step 4: Add minimal Oxlint config**

Create `gui-app/.oxlintrc.json`:

```json
{
  "$schema": "./node_modules/oxlint/configuration_schema.json",
  "ignorePatterns": ["dist/**", "node_modules/**", "src-tauri/target/**"]
}
```

- [ ] **Step 5: Add minimal Oxfmt config**

Create `gui-app/.oxfmtrc.json`:

```json
{
  "$schema": "./node_modules/oxfmt/configuration_schema.json",
  "ignorePatterns": ["dist/**", "node_modules/**", "src-tauri/target/**", "package-lock.json"]
}
```

If the installed package schema path or `ignorePatterns` key differs, adjust to match the package’s documented config (prefer schema-driven keys from the installed package).

- [ ] **Step 6: Verify scripts resolve**

```bash
npm run lint -w gui-app
npm run format:check -w gui-app
```

Expected: commands run (may exit non-zero until Task 2 baseline fix). Failure mode must be real lint/format issues, not “command not found”.

- [ ] **Step 7: Commit**

```bash
git add gui-app/package.json gui-app/.oxlintrc.json gui-app/.oxfmtrc.json package-lock.json
git commit -m "$(cat <<'EOF'
chore(gui-app): add oxlint and oxfmt tooling

EOF
)"
```

---

### Task 2: Baseline format + lint autofix

**Files:**
- Modify: any `gui-app/**/*.{ts,tsx}` (and other files oxfmt/oxlint touch under `gui-app`) that need baseline cleanup

**Interfaces:**
- Consumes: scripts from Task 1
- Produces: clean tree where `format:check` and `lint` exit 0

- [ ] **Step 1: Apply format + lint fix**

```bash
npm run format -w gui-app
npm run lint:fix -w gui-app
```

- [ ] **Step 2: Verify clean**

```bash
npm run format:check -w gui-app
npm run lint -w gui-app
```

Expected: both exit 0. If oxlint reports unfixable errors, fix them manually with the smallest change that satisfies the rule.

- [ ] **Step 3: Commit**

```bash
git add -u gui-app
git commit -m "$(cat <<'EOF'
style(gui-app): apply oxfmt and oxlint baseline fixes

EOF
)"
```

---

### Task 3: Lefthook pre-commit jobs

**Files:**
- Modify: `lefthook.yml`
- Modify: `AGENTS.md` (skip env docs only if already documenting lefthook skips)

**Interfaces:**
- Consumes: `oxfmt` / `oxlint` binaries from workspace `node_modules`
- Produces: pre-commit jobs `oxfmt` and `oxlint` with `stage_fixed: true`

- [ ] **Step 1: Extend `lefthook.yml`**

```yaml
# Git hooks via lefthook (https://lefthook.dev).
# Installed by `npm install` at the repo root (`prepare` → `lefthook install`).
# Skip a job: LEFTHOOK_EXCLUDE=cargo-fmt git commit ...
# Disable all hooks: LEFTHOOK=0 git commit ...
# Emergency bypass: git commit --no-verify

pre-commit:
  commands:
    cargo-fmt:
      glob: "*.rs"
      run: rustfmt --edition 2021 {staged_files}
      stage_fixed: true
    oxfmt:
      glob: "*.{ts,tsx}"
      run: npx oxfmt {staged_files}
      root: gui-app
      stage_fixed: true
    oxlint:
      glob: "*.{ts,tsx}"
      run: npx oxlint --fix {staged_files}
      root: gui-app
      stage_fixed: true
```

If lefthook’s `root` key is unsupported or `npx` fails to resolve workspace bins, use an equivalent that runs the `gui-app` binaries, e.g.:

```yaml
run: npm exec -w gui-app -- oxfmt {staged_files}
```

and the same pattern for oxlint. Prefer whatever works after a dry run.

Update the header comment to mention `LEFTHOOK_EXCLUDE=oxfmt` / `oxlint`.

- [ ] **Step 2: Dry-run hooks**

```bash
npx lefthook run pre-commit --all-files
```

Expected: oxfmt/oxlint run without crashing (cargo-fmt may also run).

- [ ] **Step 3: Update AGENTS.md skip docs**

Ensure Learned User Preferences / Facts mention frontend exclude names alongside `cargo-fmt` if that section already documents lefthook skips.

- [ ] **Step 4: Commit**

```bash
git add lefthook.yml AGENTS.md
git commit -m "$(cat <<'EOF'
chore: lint and format staged TS/TSX on pre-commit

EOF
)"
```

---

### Task 4: Extend CI lint job

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `gui-app` scripts from Task 1
- Produces: CI `lint` job that fails on format drift or lint errors

- [ ] **Step 1: Add Node + frontend steps to `jobs.lint`**

After the existing clippy step, append:

```yaml
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: npm

      - name: Install npm dependencies
        run: npm ci

      - name: Check frontend formatting
        run: npm run format:check -w gui-app

      - name: Run frontend lint
        run: npm run lint -w gui-app
```

Keep Rust fmt/clippy steps unchanged and first.

- [ ] **Step 2: Sanity-check workflow YAML locally**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: no parse error. If PyYAML is missing, use `node -e "require('fs').readFileSync('.github/workflows/ci.yml','utf8')"` plus visual review of indentation.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: run gui-app oxfmt and oxlint in lint job

EOF
)"
```

---

### Task 5: End-to-end verification

**Files:** none (verification only)

- [ ] **Step 1: Re-run clean checks**

```bash
npm run format:check -w gui-app
npm run lint -w gui-app
```

Expected: exit 0 both.

- [ ] **Step 2: Confirm scripts are documented in package.json**

```bash
node -e "const p=require('./gui-app/package.json'); console.log(Object.keys(p.scripts).filter(k=>/lint|format/.test(k)).sort().join(','))"
```

Expected: `format,format:check,lint,lint:fix`

- [ ] **Step 3: Confirm lefthook globs**

```bash
grep -A6 'oxfmt:\|oxlint:' lefthook.yml
```

Expected: `*.{ts,tsx}` and `stage_fixed: true` present for both.
