# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Users

DJs at every level — bedroom practice through club and mobile live performance — who want professional mixing tools without vendor lock-in, subscription paywalls, or hardware gatekeeping. Secondary audience: developers who fork, extend, or embed the open-source Rust audio engine.

## Product Purpose

Mixar is open-source DJ software: dual decks, a mixer, a track library, and performance tools (hot cues, loops, pads, sampler, beat sync) on a low-latency Rust audio engine. Success means reliable live mixing, a complete booth workflow, and a codebase contributors can inspect and extend. The product is under active development and is not a released product yet.

## Positioning

A modular Rust audio engine with runtime-selectable backends (CPAL/PipeWire on Linux, miniaudio, null for tests), a headless producer-thread + lock-free ring-buffer architecture, and a Flutter desktop host — shipped as GPL-3.0 software with no features hidden behind a paywall. Cross-platform (desktop and mobile) is a first-class product goal, not a port of a desktop-only app.

## Operating Context

- **Primary dev platform:** Linux x86_64; desktop UI via Flutter (`apps/gui-flutter`), engine in Rust (`crates/`).
- **Typical session:** Load tracks from folder collections, analyze BPM/key offline, perform with dual decks, mixer (gain/EQ/filter/crossfader/cue), hot cues, loops, and MIDI controllers.
- **Library storage:** `{applicationSupport}/library.db` (app id `top.mixar.app`).
- **Set history:** Session logging to XSPF under app support; external tools (e.g. OBS) can watch active session files.
- **Marketing site:** Static Astro site at `apps/website` — landing page and developer guide; not the mixing product itself.

## Capabilities and Constraints

**Shipped or in progress (confirmed in repo):**

- Dual decks: overview + scrolling waveforms, beat grid, play/pause, seek, tempo
- Mixer: gain, 3-band EQ, filter, volume, crossfader, cue/PFL, VU meters
- Hot cues, loops, performance pads, sampler, beat sync
- Library with folder collections, metadata, offline analysis (BPM, key, loudness, artwork)
- MIDI controller mappings
- Default engine: 48 kHz, 512-frame buffers; latency tied to buffer size
- Engine/library/controller I/O via MessagePack buses; Flutter bridges via FRB transports

**Platform status:**

- **Linux:** primary development and current shipping target for desktop
- **macOS, Windows, iOS, Android:** on roadmap (not yet available)

**Explicit non-goals / deferred (MVP scope):**

- Semitone key shift / stems beyond key-lock tempo stretch
- Recording/streaming, telemetry
- WASAPI exclusive / ASIO native low-latency backends (v2 target)
- WASM browser mixing (future work)

**License:** GPL-3.0

**Terminology:** "Deck" = one loaded track's performance surface; "Collection" = folder or playlist in the library; engine hosts must not hold session locks while waiting on library I/O.

## Brand Commitments

- **Name:** Mixar
- **Repository:** https://github.com/geovannimp/mixar
- **Voice (marketing):** Direct, respectful of the DJ — professional tools without lock-in; "no paywall surprises"; software for how you mix today
- **Status badge:** "In development" on the landing page until a real release ships
- **Open source:** GPL-3.0; contributions welcome at every skill level

## Evidence on Hand

| Asset | Location |
|-------|----------|
| Product screenshot / hero | `docs/mixar-banner.png`, `apps/website/public/mixar-banner.png` |
| Technical architecture | `docs/tech-spec.md` |
| Deck UI roadmap & data model | `docs/deck-spec.md` |
| Waveform & analyzer specs | `docs/dj-waveform-spec.md`, `docs/audio-analyzer-spec.md` |
| Set history spec | `docs/history-spec.md` |
| Sample audio | `samples/` |
| CI / build badge | GitHub Actions on `main` |

**Do not fabricate:** customer testimonials, download counts, benchmark claims, pricing tiers, licensing beyond GPL-3.0, or platform availability beyond what is shipped or explicitly on the roadmap.

## Product Principles

1. **No lock-in** — open source, inspectable code, MIDI for any controller, no paywalled "pro" features.
2. **Performance first** — low-latency engine, tight mixer feel; timing matters in live sets.
3. **Modular by design** — small Rust crates, pluggable backends and hosts; engine usable headless without the GUI.
4. **Cross-platform from the start** — one product shaped for every screen you mix on, not a desktop app awkwardly ported elsewhere.
5. **Honest shipping** — mark in-development status clearly; do not claim features or platforms that are not built yet.

## Accessibility & Inclusion

No product-specific standard established yet. Best-effort accessibility (semantic markup, keyboard focus, reduced-motion respect on web) until requirements are defined.
