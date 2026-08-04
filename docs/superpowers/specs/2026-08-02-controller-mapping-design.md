# Controller mapping (MIDI bundles)

Date: 2026-08-02  
Issue: [#49](https://github.com/geovannimp/rust-dj-engine/issues/49)  
Spec refs: `docs/deck-spec.md` §5.16 HW1, §9.8; `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md`  
Status: implemented (v1 runtime; host/Tauri MIDI I/O follow-up)

## Goal

Ship an **open, shareable controller-mapping API** in the spirit of Mixxx and VirtualDJ: users (and a future marketplace) can add mappings without waiting for an app release. Controllers talk to the engine only through the existing **cmd/evt bus** (`engine-api` `Origin` / `Kind` / payloads) — same path as the UI.

Inspired by:

- [Mixxx MIDI mapping + scripting](https://github.com/mixxxdj/mixxx/wiki/MIDI-controller-mapping-file-format) (declarative XML + JS)
- [VirtualDJ device definition + mapping](https://virtualdj.com/wiki/Controller%20Developers.html) (MIDI→names, names→script)
- Community map hubs such as [DJ TechTools Midimaps](https://maps.djtechtools.com/) (file-first multi-software ecosystem)

Scripting language: [Rhai](https://github.com/rhaiscript/rhai) (Rust-native, sandboxed, WASM-capable).

## v1 scope

| In | Out (later phases) |
|----|---------------------|
| Load/run mapping **bundles** | MIDI learn UI + export |
| MIDI in → cmd bus; evt → MIDI out (LEDs) | Mixxx / VirtualDJ importers |
| Autoload by device identity | Marketplace |
| Declarative modifiers + soft-takeover | HID profiles (HW2) |
| Optional `script.rhai` hooks | Motorized faders (HW3) |
| `map-check` + moon tasks + TOML Schema | Full sysex protocol helpers |
| One example bundle | Per-bundle golden MIDI fixtures (optional later) |

## Approach

**Data-driven `map.toml` + thin Rhai host API.**  
1:1 bindings are lookup tables. Rhai runs only for lifecycle hooks and explicit `script = "fn"` overrides. No Mixxx-style Control Object store in v1 (evt snapshot cache is enough); converters can target our closed action/alias catalogs later without restructuring.

## Architecture

```text
┌─────────────┐     MidiPort trait      ┌──────────────────┐
│ Host adapter│◄───────────────────────►│ OS MIDI / WebMIDI│
│ (Tauri now, │                         └──────────────────┘
│  WASM later)│
└──────┬──────┘
       │ uses
       ▼
┌──────────────────┐   publish cmd    ┌─────────────┐
│ controller crate │ ───────────────► │ engine-api  │
│ bundle load      │   subscribe evt  │ Origin/Kind │
│ alias resolve    │ ◄─────────────── │ omnibus     │
│ map.toml tables  │                  └─────────────┘
│ script.rhai hooks│
└──────────────────┘
```

| Piece | Role |
|-------|------|
| **Bundle** | Folder: `device.toml` + `map.toml` + optional `script.rhai` |
| **`controller` crate** | Parse/validate, match device, resolve aliases, apply map, run Rhai; **WASM-safe** (no OS MIDI inside) |
| **Host adapter** | Enumerate/connect ports (`midir` desktop / Web MIDI WASM), feed bytes in, send bytes out, bridge bus |
| **Engine** | Unchanged; mapper is another cmd publisher / evt subscriber |

**Rules:**

- `device.toml` is **required** (identity, aliases, optional audio hints) so bundles can autoload and later bind controller audio.
- MIDI I/O never runs on the audio callback.
- Mapper publishes with normal `Origin::Deck(n)` / `Mixer` / `Engine` (same as UI). Optional debug tagging can wait.
- Workspace layout: `mappings/<id>/` for shipped and user-shareable packages (e.g. `mappings/ddj-400/`).

## Bundle format

```text
mappings/ddj-400/
  device.toml      # required
  map.toml         # required
  script.rhai      # optional
```

### `device.toml`

Identity for autoload, optional audio hints, sectioned MIDI aliases.

```toml
schema_version = 1
id = "pioneer.ddj-400"
name = "Pioneer DDJ-400"
usb_vid = 0x2b73
usb_pid = 0x0014
midi_name_contains = ["DDJ-400"]

[audio]
output_name_contains = ["DDJ-400"]
# channel roles: reserved; v1 host may use name hints only

[deck_1]
play_pause = { type = "note", channel = 1, note = 0x0B }
volume     = { type = "cc",   channel = 1, cc = 0x13 }
jog_touch  = { type = "note", channel = 1, note = 0x36 }
jog_turn   = { type = "cc",   channel = 1, cc = 0x21 }
# Output-only endpoints may reuse the same note as an input (LED on the button).
play_led   = { type = "note", channel = 1, note = 0x0B, velocity = 0x7F, direction = "out" }
pause_led  = { type = "note", channel = 1, note = 0x0C, velocity = 0x7F, direction = "out" }

[deck_2]
play_pause = { type = "note", channel = 2, note = 0x0B }

[master]
crossfader = { type = "cc", channel = 1, cc = 0x1F }

[custom]
shift = { type = "note", channel = 1, note = 0x3F }
```

**MIDI message table (v1):** `type` = `note` | `cc` | `cc14`. Include `channel` and:
- `note` for notes
- `cc = <u8>` for 7-bit CC
- `cc = { msb = …, lsb = … }` for 14-bit CC (session pairs both bytes → one 0…1 value)
Optional `velocity` / `value` for defaults on output. Optional `direction` = `in` | `out` | `inout` (default `inout`). **Input matching** only considers aliases with `in` or `inout`. Shared note/CC across an input alias and an `out`-only LED alias is normal and allowed. `cc14` registers both MSB and LSB as input identities for the same alias.

**Autoload:** on port connect, match `usb_vid`/`usb_pid` and/or `midi_name_contains` → load bundle → `on_init` → enable map. Pinning an alternate map for the same device is settings work after v1.

### Alias catalog (closed + `custom` + output endpoints)

| Section | Keys |
|---------|------|
| `deck_N` (`N` = 1..4 in v1) | **Input/watch catalog (closed):** `play_pause`, `cue`, `cue_hold`, `sync`, `quantize`, `volume`, `gain`, `eq_high`, `eq_mid`, `eq_low`, `filter`, `jog_touch`, `jog_turn`, `hot_cue_1`…`hot_cue_8`, `pad_1`…`pad_8`, plus pad-mode / loop / beat-jump names needed for existing `Kind`s. **Extra device endpoints:** any other `snake_case` name is allowed in `device.toml` only as MIDI endpoints (typically `direction = "out"`) referenced from `map.toml` outputs — not as `[inputs.*]` keys. |
| `master` | Closed: `crossfader`, `cue_mix`, `master_cue`, headphone-related keys aligned with mixer cmds; plus optional out-only endpoint names |
| `sampler` | Optional; slot/bank keys aligned with sampler cmds |
| `custom` | Any `snake_case` name; **not** bindable to declarative engine actions — modifiers / Rhai only |

- `[inputs.*]` / output **watch** keys must be in the closed catalog for that section → else **load error**.
- Output `on`/`off` alias strings must name some endpoint in `device.toml` (catalog or extra) → else **load error**.
- Deck index from section: `deck_1` → `Origin::Deck(0)`.
- Full closed lists live in code + `.tosd` (single source of truth in the `controller` crate; schemas generated or kept in sync in the implementation plan).

### `map.toml` (bidirectional)

```toml
schema_version = 1

[inputs.deck_1]
play_pause = "toggle_play"
volume = [
  { action = "set_volume", soft_takeover = true },
  { action = "set_filter", modifier = "custom.shift", soft_takeover = true },
]
jog_turn = "jog_turn"
# play_pause = { script = "on_play_pause" }

[inputs.master]
crossfader = { action = "set_crossfader", soft_takeover = true }

[outputs.deck_1]
# Alias targets (MIDI from device.toml) — optional form
play_pause = { on = "pause_led", off = "play_led" }
# Inline MIDI — also valid
cue = {
  on  = { type = "note", channel = 1, note = 0x0C, velocity = 0x7F },
  off = { type = "note", channel = 1, note = 0x0C, velocity = 0x00 },
}
```

**Inputs**

- Key = catalog alias under a section (`[inputs.deck_1]` …).
- Value = action string, single binding table, or **array of bindings** (TOML cannot duplicate keys).
- Binding fields: `action`, optional `modifier` (`custom.*` alias), optional `soft_takeover`, optional `script` (mutually exclusive with `action` for that binding).

**Modifier priority:** when MIDI arrives for an alias, prefer a binding whose `modifier` is currently active over an unmodified binding. If multiple modifier bindings could match, **first listed wins** (v1). Advanced cases use Rhai.

**soft_takeover:** for absolute controls (faders/knobs), ignore MIDI until hardware value is within **3/127** of the current engine value (normalized 0..1) or crosses it, then latch. Prevents jumps after load, UI moves, or layer changes. Default: off for buttons; **on** for absolute CC actions unless overridden.

**Outputs**

- Watch an engine signal tied to the catalog alias (e.g. `play_pause` → playing bool).
- `on` / `off` each resolve to either a **device alias string** or an **inline MIDI** table. Aliases are optional convenience, not required.

### Action vocabulary

Closed strings in `map.toml` (e.g. `toggle_play`, `set_volume`, `set_filter`, `jog_turn`, `trigger_hot_cue_1`, …). Each expands to an existing `Kind` + `CmdBody` (no actions without an engine cmd). Unknown action → load error. Exact list is owned by the `controller` crate beside `Kind`; grows with the engine; Mixxx/VDJ converters target this list later.

### `script.rhai` (optional)

- Hooks: `on_init(ctx)`, `on_shutdown(ctx)`.
- Named functions for `script = "..."` bindings.
- v1 host API: `publish(origin, kind, payload)`, `midi_out(bytes)`, `get_snapshot()` (last Status/Updated cache), script-local vars.
- `custom.*` state (e.g. shift held) is readable for complex logic; declarative modifiers cover the common case without script.

## Runtime data flow

```text
MIDI bytes (host)
  → parse short message (v1)
  → match device.toml alias
  → if [custom]: update modifier/script state; continue only if also mapped as input
  → resolve map.toml inputs
       · modifier-active binding wins over unmodified
       · soft_takeover gate for absolute controls
       · action → Origin + Kind + CmdBody  OR  Rhai fn
  → publish cmd bus

Engine evt bus
  → update snapshot cache
  → outputs.* whose watched signal changed → resolve on/off (alias or inline) → midi_out
  → Rhai may publish / midi_out from hooks
```

**Coalescing:** CC ≤ ~60 Hz per `(section, alias)` before publish (deck-spec). Notes/buttons: edge-triggered.

## Errors

| Case | Behavior |
|------|----------|
| Parse / unknown catalog key / unknown action / bad schema_version | Fail load; host surfaces error; device unmatched |
| Rhai compile error / missing script when referenced | Fail load |
| Rhai runtime error | Log + evt `Notice`/`Error`; skip that event; do not crash host |
| Soft-takeover blocking | Silent (no cmd) until latched |
| MIDI port gone | `on_shutdown`; re-autoload on reconnect |
| No matching bundle | Ignore MIDI (log once) |

## Validation & tooling

### TOML Schema ([toml-schema.org](https://toml-schema.org/))

| File | Purpose |
|------|---------|
| `schemas/device.tosd` | Identity, audio hints, sectioned aliases, MIDI message shapes |
| `schemas/map.tosd` | inputs/outputs, actions, modifiers, soft_takeover, alias-or-inline unions |

Editors use these for validation and autocomplete. Closed catalogs / actions appear as `allowedvalues` (or equivalent) where the schema language allows.

### `map-check` (controller crate)

Static/load-time checks (no hardware):

- TOML parse + schema/structural rules
- Unknown catalog keys; unknown actions
- Alias references (`modifier`, output `on`/`off` strings) missing from `device.toml`
- Rhai compile; missing `script.rhai` when required
- Two **input-eligible** aliases (`direction` in/inout) claiming the same type/channel/note|cc → **error**; sharing with `direction = "out"` is OK
- Unsupported `schema_version`

Semantic cross-refs and Rhai stay in `map-check` even when `.tosd` covers structure.

### Moon / CI

```text
moon run controller:test-mappings           # all mappings/<id>/  → CI
moon run controller:test-mapping -- ddj-400 # one id             → local
```

Optional root npm aliases (`test:mappings`, `test:mapping`) for shorter DX. CI runs `controller:test-mappings` with the rest of `:test`.

## Testing

| Layer | What |
|-------|------|
| Unit | Parse; alias match; modifier priority; soft-takeover; action→Kind; output alias vs inline |
| Script | Rhai hooks with mock publish/midi_out |
| Integration | Fake `MidiPort` + in-process/memory bus → input cmd + LED out |
| map-check | Fixture bundles that must pass/fail named checks |
| Host | Manual smoke: real port + example bundle |

## Later phases (API-stable intent)

1. **MIDI learn + export** — write/update bundles from UI; export for sharing.
2. **Importers** — Mixxx XML(+JS subset) and/or VDJ device+map → our bundle (best-effort; Rhai/JS parity incomplete).
3. **Marketplace** — distribute bundles; `map-check` + schema as gate.
4. **HID (HW2)** — extend device layer; keep map/action catalogs.
5. **Control Object layer** — only if importers need Mixxx fidelity beyond action catalogs.

## Acceptance (v1)

- [x] `controller` crate loads a valid bundle and rejects invalid ones with clear errors
- [x] Host adapter can attach a MIDI port, autoload by identity, publish play/volume-class cmds on the engine bus *(Tauri `ControllerEngine` host; midir git-pin for alsa/cpal coexistence; connect asks FE before enable)*
- [x] Output path lights LEDs from evt (alias and inline forms)
- [x] Modifier + soft-takeover behave as specified
- [x] Optional Rhai `on_init` / script binding works
- [x] `schemas/*.tosd` exist for device + map
- [x] `moon run rust:test-mappings` passes; `moon run rust:test-mapping -- <id>` works locally
- [x] One example bundle under `mappings/`
