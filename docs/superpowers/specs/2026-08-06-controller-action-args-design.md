# Controller map action named arguments

Date: 2026-08-06  
PR: [#132](https://github.com/geovannimp/rust-dj-engine/pull/132)  
Related: `docs/superpowers/specs/2026-08-03-qualified-controller-actions-design.md`  
Follow-up: [#137](https://github.com/geovannimp/rust-dj-engine/issues/137) (beats → f32)  
Status: approved; implementing

## Goal

Replace numeric/mode suffixes in action leaves (`pad_3`, `load_to_deck_1`, `pad_mode_hot_cue`) with named arguments on the leaf: `pad(n:3)`, `load_to_deck(deck:1)`, `pad_mode(mode:hot_cue)`.

## Grammar

```text
<OriginTemplate>::<leaf>
<OriginTemplate>::<leaf>(<key>:<value>, ...)
```

- Named args only (no positional).
- Empty `()` forbidden — use bare leaf when there are no params.
- Values this pass: signed integers (`-2`, `4`) or bare idents (`hot_cue`). No quotes.
- Fractional beats (`1/4`) deferred to [#137](https://github.com/geovannimp/rust-dj-engine/issues/137).
- Keys closed per leaf; unknown / missing required → load/`map-check` error.
- Indices in TOML are **1-based** labels (`deck:1`, `n:1`, `slot:1`); resolve converts to 0-based for the wire where needed.
- No upper caps on `deck` / `n` / `slot` / `beats` (engine may still reject).
- Hard break: old `_N` / `pad_mode_*` / `beat_jump_fwd_*` spellings fail validation.

## Leaf schemas

| Leaf | Args | Notes |
|------|------|--------|
| `load_to_deck` | `deck:N` (`N ≥ 1`) | Library `Load { deck: N-1 }` |
| `pad` | `n:N` (`N ≥ 1`) | Software pad bank routing |
| `trigger_hot_cue` | `slot:N` (`N ≥ 1`) | |
| `delete_hot_cue` | `slot:N` (`N ≥ 1`) | |
| `trigger_sampler` | `slot:N` (`N ≥ 1`) | |
| `auto_loop` | `beats:N` (`N ≥ 1`, integer) | Bare `auto_loop` (no args) stays if already a distinct action |
| `beat_jump` | `beats:N` (`N ≠ 0`, signed int) | Replaces `beat_jump_fwd` / `beat_jump_back` (+ `_N`) |
| `pad_mode` | `mode:` ∈ {`hot_cue`,`loop_roll`,`beat_jump`,`sampler`} | |

No-arg leaves unchanged: `navigate`, `navigate_next`, `navigate_prev`, `toggle_play`, etc.

Device **aliases** (`hot_cue_1`, `loop_pad_3`, `load_deck_1`) stay as hardware names; only **action** strings migrate.

## Runtime

`parse_action_id` → `(OriginTemplate, leaf, args)`. Catalog matches **base** leaf + schema. `resolve_action` reads typed args; same `RoutedAction` outputs as today.

## Migrations

- `mappings/ddj-400/map.toml` + controller fixtures
- Update qualified-actions design note
- Tests for parse, reject old forms, resolve sample actions
