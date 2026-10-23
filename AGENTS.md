## Code navigation

- Always prefer CodeGraph (`codegraph_explore`) over grep/find or Read for any **structural** question — how does X work, how does X reach Y, what calls what, where is X defined, or surveying an area. One call returns verbatim line-numbered source grouped by file (safe to Edit from), the call paths among those symbols (including dynamic-dispatch hops grep can't follow), and a blast-radius summary of what depends on them.
- Answer directly; don't delegate exploration. CodeGraph is the pre-built index, so spawning a file-reading sub-agent or running a grep + read loop repeats work it already did. Follow up with another `codegraph_explore` naming more specific symbols if needed.
- Trust the results — they come from a full AST parse. Do not re-verify them with grep.
- Index lag: if a response starts with "⚠️ Some files referenced below were edited since the last index sync…", Read only the files listed there; everything else in the response is fresh and authoritative.
- Query this repo's index by passing the project path (`.codegraph/` at the repo root; the server has no default project). Fall back to `read`/`grep`/`glob` only for what CodeGraph does not index (docs, configs, `mappings/`, manifests) or to confirm a specific detail it did not cover.
- If a checkout has no `.codegraph/`, ask the user before running `codegraph init -i`.

## Docs

- Before architecture, module, or crate-boundary work: read `docs/tech-spec.md` and `DEVELOPER.md`.
- Before deck UI, data-model, or phased feature work: read `docs/deck-spec.md`.
- Subsystem references (MIDI mappings, set history, stems, …): `apps/website/src/content/docs/developers/`. User-facing product docs: `apps/website/src/content/docs/users/`.
- Follow those docs instead of inventing parallel designs. Point at them; do not restate their contents here.
- For other day-to-day details (audio backends, host transports, library DB, tooling), use `README.md` / `DEVELOPER.md` / `docs/*` — not this file.

## Process

- The website dev server may already be running (`astro dev`, usually port 4321) with live reload. Do not stop or restart it; let HMR pick up edits and verify against the running server.
- Prefer minimal, focused changes; scope may narrow mid-task.
- When planning features, research how competitors handle the same problem before proposing UX or architecture.
- Always run the relevant tests (and lint/format as needed) before committing.
- Before opening a PR, invoke the `open-code-review-delegate` skill and run its review; fix every reported issue, then rerun the review and relevant tests until clean. Delegation uses OCR for deterministic file selection and rules, and the host agent performs the review without requiring OCR LLM credentials.
- For PRs, use OpenCodeReview's SARIF output when publishing GitHub Code Scanning results; CodeGraph is preferred for structural code questions, while `code_search` is for text search.
- After pushing, check CI and fix failures before considering the work done.

## Guardrails

- `engine-dsp` stays pure DSP: no I/O, filesystem, network, codec, or backend deps.
- Audio callback / `AudioCallback::render`: no heap allocations, mutexes, or blocking.
- Flutter UI never calls raw host/FRB wire APIs from widgets — use `EngineTransport`, `LibraryTransport`, `ControllerTransport` (and Riverpod). Do not hand-edit generated FRB under `apps/gui-flutter/lib/src/rust/`; regenerate with `moon run gui-flutter:generate`.
- Prefer Mixar shell widgets / `context.theme` tokens and concise toasts (`showMixarToast`) over one-off chrome and verbose status text.

## Mappings

- When changing any shipped MIDI mapping bundle under `mappings/<id>/`, always bump that bundle’s `mapping_version` in `device.toml` to a strictly newer parseable value (e.g. `1.0.0` → `1.0.1`). Lint enforces this on PRs via `scripts/check-mapping-version-bump.sh` so Settings can show “Update available.”
