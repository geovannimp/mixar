# MessagePack Engine Wire Codec

**Date:** 2026-07-26  
**Parent:** `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md`

## Goal

Replace postcard with **MessagePack (named maps)** end-to-end for bus wire bytes, and drive TS encode/decode through **Zod codecs** wrapping pack/unpack. Keep the envelope shape and helper names; drop the hand-written postcard binary codec in the GUI.

## Decisions

| Topic | Choice |
|-------|--------|
| Wire format | MessagePack |
| Struct encoding | Named maps (`rmp_serde::to_vec_named` / equivalent on decode) |
| Envelope | `{ origin, kind, revision, body: bytes }` — body is MessagePack of cmd/evt payload |
| TS API | Keep `encodeWire` / `decodeEvtBody` / …; codecs exported for tests |
| Zod role | `z.codec(Uint8Array, ValueSchema, { decode: unpack+parse, encode: parse+pack })` |
| Compat | Breaking on this branch; no dual-codec period |

## Serde ↔ Zod shape

- `Origin` / `Kind`: `rename_all = "snake_case"` (`Deck(n)` → `{"deck": n}`; kinds as `"play"`, …).
- `CmdBody` / `EvtBody`: `#[serde(tag = "type", rename_all = "snake_case")]`.
- `EvtBody::EngineStatus` is a struct variant `{ status: EngineStatus }` so TS keeps `{ type: "engine_status", status: { … } }`.

## Crates / packages

- Rust `engine-api`: `rmp-serde`; remove `postcard`.
- TS `gui-app`: `@msgpack/msgpack` + existing `zod` (Zod codecs wrap pack/unpack).

## Tests

- Regenerate `play_deck1.hex` from Rust encode of Play / Deck(1) / Empty / revision 0.
- Rust roundtrip + TS golden encode/decode parity.

## Out of scope

- Composed primitive codecs; flattened envelope; library invokes; dual postcard/MessagePack.
