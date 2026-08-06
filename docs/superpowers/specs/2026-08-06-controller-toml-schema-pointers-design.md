# Controller mapping TOML Schema pointers (`.tosd`)

Date: 2026-08-06  
PR: [#132](https://github.com/geovannimp/rust-dj-engine/pull/132) (review thread on `device.toml` `[toml-schema]`)  
Related: `docs/superpowers/specs/2026-08-02-controller-mapping-design.md` § Validation & tooling  
Status: approved design (not yet implemented)

## Goal

Wire mapping data files to editor-facing [TOML Schema](https://toml-schema.org/) documents (`.tosd`), and refresh those schemas to match today’s `device.toml` / `map.toml` shape (`vendor_name` / `product_name`, etc.). Runtime validation stays Rust + `map-check`.

## Non-goals

- Running a toml-schema CLI/crate in CI or `map-check` this pass
- Encoding closed alias / action catalogs in `.tosd` (those stay in Rust)
- Changing app `schema_version = 1` semantics
- Committing GitHub absolute `location` URLs yet (relative paths first; remote URLs are a later swap)

## Approach

1. Add reserved `[toml-schema]` metadata to every shipped and fixture `device.toml` / `map.toml`.
2. Rewrite `schemas/device.tosd` and `schemas/map.tosd` to the real language (`[toml-schema]` + `[elements]` / `[types]`).
3. Teach Rust parsers to **deserialize and ignore** `[toml-schema]` so `DeviceFile`’s `#[serde(flatten)]` sections map does not treat it as a MIDI section.

## Pointers

Every data file includes:

```toml
[toml-schema]
version = "1.0.0"
location = "<path-relative-to-this-file>"
```

| `version` | TOML Schema **language** SemVer (not `schema_version = 1`) |
| `location` | Relative URI resolved from the referencing TOML file |

Examples:

| File | `location` |
|------|------------|
| `mappings/<id>/device.toml` | `../../schemas/device.tosd` |
| `mappings/<id>/map.toml` | `../../schemas/map.tosd` |
| `crates/controller/tests/fixtures/<id>/device.toml` | `../../../../schemas/device.tosd` |
| `crates/controller/tests/fixtures/<id>/map.toml` | `../../../../schemas/map.tosd` |

Later: replace `location` with a GitHub raw/HTTPS URL; no Rust change required if the field remains an opaque string.

## Rust ignore field

`DeviceFile` and `MapFile` gain an optional field (names illustrative):

```rust
#[serde(default, rename = "toml-schema")]
toml_schema: Option<TomlSchemaRef>,

struct TomlSchemaRef {
    version: Option<String>,
    location: Option<String>,
}
```

Never read after deserialize. Without this, `[toml-schema]` lands in `sections` (or fails untagged MIDI endpoint decode) and breaks load.

`DeviceFile::parse` already retains only known section keys; the typed field is still required so serde does not attempt to parse `version`/`location` as `MidiEndpoint`.

## `.tosd` rewrite

Replace the stale `title` / `[[properties]]` drafts with valid TOML Schema documents.

### `schemas/device.tosd`

Structural only:

- Required: `schema_version` (integer, allowed `1`), `id`, `vendor_name`, `product_name`
- Optional: `description`, `usb_vid`, `usb_pid`, `midi_name_contains` (string array), `audio` (table)
- Section tables `deck_1`…`deck_4`, `master`, `sampler`, `custom` as collections of MIDI endpoint tables (dynamic alias keys)
- Reusable `types.midi_endpoint`: `type` ∈ `note`/`cc`/`cc14`, `channel` 1..=16, optional `note`/`cc`/`velocity`/`value`/`direction`

Do not require documenting `[toml-schema]` under `[elements]` — validators ignore the reserved metadata table when omitted from the schema.

### `schemas/map.tosd`

Structural only:

- Required: `schema_version` (integer, allowed `1`)
- `inputs` / `outputs` as tables of section → alias entries
- Binding as `oneof`: action string | binding table | array of binding tables
- Binding table optional fields: `action`, `modifier`, `soft_takeover`, `script`

Closed action strings and alias catalogs remain comments or Rust/`map-check`, not `allowedvalues` churn in `.tosd`.

## Still validated only in Rust / `map-check`

- Closed input aliases and qualified actions
- MIDI clash among input-eligible endpoints
- Modifier / output alias cross-refs
- Rhai compile
- Unsupported app `schema_version`

## Testing

- Existing fixture + shipped map loads must keep passing with `[toml-schema]` present
- At least one unit/fixture assert that a file with `[toml-schema]` parses (serde ignore)
- No new CI job for external `.tosd` validation this pass

## PR / docs follow-through

- Resolve the `#132` `[toml-schema]` review thread after land
- Patch the Validation & tooling note in `2026-08-02-controller-mapping-design.md` if it still describes the old `.tosd` draft shape (one-line pointer to this spec is enough)
