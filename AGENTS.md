## Docs

- Before architecture, module, or crate-boundary work: read `docs/tech-spec.md` and `DEVELOPER.md`.
- Before deck UI, data-model, or phased feature work: read `docs/deck-spec.md`.
- Subsystem references (MIDI mappings, set history, stems, …): `apps/website/src/content/docs/developers/`. User-facing product docs: `apps/website/src/content/docs/users/`.
- Follow those docs instead of inventing parallel designs. Point at them; do not restate their contents here.
- For other day-to-day details (audio backends, host transports, library DB, tooling), use `README.md` / `DEVELOPER.md` / `docs/*` — not this file.

## Process

- Prefer minimal, focused changes; scope may narrow mid-task.
- When planning features, research how competitors handle the same problem before proposing UX or architecture.
- Always run the relevant tests (and lint/format as needed) before committing.
- After pushing, check CI and fix failures before considering the work done.

## Guardrails

- `engine-dsp` stays pure DSP: no I/O, filesystem, network, codec, or backend deps.
- Audio callback / `AudioCallback::render`: no heap allocations, mutexes, or blocking.
- Flutter UI never calls raw host/FRB wire APIs from widgets — use `EngineTransport`, `LibraryTransport`, `ControllerTransport` (and Riverpod). Do not hand-edit generated FRB under `apps/gui-flutter/lib/src/rust/`; regenerate with `moon run gui-flutter:generate`.
- Prefer Mixar shell widgets / `context.theme` tokens and concise toasts (`showMixarToast`) over one-off chrome and verbose status text.

## Mappings

- When changing any shipped MIDI mapping bundle under `mappings/<id>/`, always bump that bundle’s `mapping_version` in `device.toml` to a strictly newer parseable value (e.g. `1.0.0` → `1.0.1`). Lint enforces this on PRs via `scripts/check-mapping-version-bump.sh` so Settings can show “Update available.”
