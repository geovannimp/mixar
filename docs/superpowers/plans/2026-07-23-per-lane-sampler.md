# Per-lane Sampler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or implement inline). Steps use checkbox (`- [ ]`) syntax.

**Goal:** Own a `Sampler` per `MixerLane`, route before/after strip (default before), deck-scoped engine APIs so two decks can play the same sample.

**Architecture:** Lane owns sampler + strip route; mixer no longer mixes a shared sampler. App tracks loaded bank + slot UI cache per deck.

**Tech Stack:** Rust `engine-dsp` / `engine-core`, Tauri settings, React settings UI.

## Global Constraints

- Default strip route: **before**
- Per-lane sampler (independent voices)
- No library schema changes

---

### Task 1: DSP — route enum + MixerLane owns Sampler

- [ ] Add `SamplerStripRoute { BeforeStrip, AfterStrip }` (default Before)
- [ ] Move `Sampler` into `MixerLane`; `begin_render` / `process` implement before/after
- [ ] Remove mixer-level sampler; `set_normalizer_target` updates all lane samplers
- [ ] `DspEngine::sampler(deck_id)` helpers
- [ ] Unit test: before mixes into dry path; after adds post-strip
- [ ] `cargo test -p engine-dsp`

### Task 2: engine-core deck_id APIs

- [ ] All sampler methods take `deck_id`
- [ ] `set_sampler_strip_route(route)`
- [ ] `cargo check -p engine-core`

### Task 3: App — per-deck load + settings

- [ ] `loaded_sampler_bank_id: [Option<String>; NUM_DECKS]`, `sampler_slots` per deck
- [ ] Pass `deck_id` through assign/trigger/clear/play-mode/load_bank
- [ ] Settings: `sampler_strip_route: "before" | "after"`
- [ ] Settings UI control
- [ ] Status exposes slots for active bank per deck (or both in sampler status keyed by deck — simplest: `SamplerStatus` stays for “focused” deck 0 in shared panel fields, but each `DeckPadsPanel` gets that deck’s slots from `deck`-keyed cache)
- [ ] Verify compile

### Task 4: Spec status + engine-dsp rule

- [ ] Mark design spec accepted; update `.cursor/rules/engine-dsp.mdc`
