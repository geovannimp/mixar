# Controller mapping lifecycle section in `map.toml`

Date: 2026-08-06  
PR: [#132](https://github.com/geovannimp/rust-dj-engine/pull/132)  
Related: `docs/superpowers/specs/2026-08-02-controller-mapping-design.md` § `script.rhai`  
Status: approved; implemented

## Goal

Map lifecycle events to Rhai function names in `map.toml` instead of hardcoding `on_init` / `on_shutdown` / `idle_heartbeat` in the session.

## Contract

Optional table:

```toml
[lifecycle]
on_init = "on_init"
idle_heartbeat = "idle_heartbeat"
on_shutdown = "on_shutdown"
```

| Rule | Behavior |
|------|----------|
| No `[lifecycle]` | No lifecycle hooks run (even if `script.rhai` defines them) |
| Partial table | Only listed events fire |
| Values | Non-empty Rhai function name strings |
| Keys | Closed set: `on_init`, `on_shutdown`, `idle_heartbeat` |
| Unknown keys | `map-check` / load validation error |

`idle_heartbeat` cadence stays Rust `IDLE_HEARTBEAT_INTERVAL` (1s) this pass — not configurable in TOML.

Missing fn at call time: keep today’s “function not found → Ok” soft miss (optional hooks). `map-check` should warn or error if a declared name is absent from the compiled AST when `script.rhai` is present.

## Runtime

`MappingSession::on_init` / `on_shutdown` / `idle_heartbeat` look up the fn name from `bundle.map.lifecycle` and call that; if the key is absent, return Ok without calling Rhai.

## Migrations

- `mappings/ddj-400/map.toml` — add full `[lifecycle]`
- `crates/controller/tests/fixtures/with-script/map.toml` — add `[lifecycle]`
- Other fixtures without scripts — leave section omitted
- `schemas/map.tosd` — optional `lifecycle` with the three optional string fields
- Update controller-mapping design doc hook note

## Testing

- Fixture with `[lifecycle]` still fires `on_init` midi_out
- Bundle with `script.rhai` but **no** `[lifecycle]` does **not** call hooks
- Unknown lifecycle key fails validation
