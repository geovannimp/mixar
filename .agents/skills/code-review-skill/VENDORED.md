# Vendored skill: code-review-skill

Upstream: <https://github.com/awesome-skills/code-review-skill> (MIT — see `LICENSE`)
Pinned revision: `4850184dfd7765ea7b3c5c4e9b6cf8e0815ee7cb`

Trimmed to what this repository needs: the languages it actually contains (Rust,
TypeScript, Dart/Flutter) plus the language-agnostic guides.

**Removed** — every other language guide, the web-performance review guide, the XSS
cross-cutting guide, the review-process/best-practices docs, the PR comment template,
the Python scripts, and the upstream site files (`README.md`, `index*.html`,
`CONTRIBUTING.md`, `.hive/`).

**Changed** — `SKILL.md`'s `description` now advertises only the guides that ship, its
`allowed-tools` list is narrowed to `Read`, `Grep`, `Glob` (the tools this project's
review session actually loads), and cross-references to removed files were de-linked
so no reference chases a file that is not here. The retained prose is unchanged.

**To re-sync** — `npx skills add awesome-skills/code-review-skill`, diff against this
directory, and re-apply the trim.
