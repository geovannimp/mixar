# Waveform Viewport Ring — Design

**Date:** 2026-09-14  
**Status:** Approved (GUI scrolling lane)  
**Related:** `docs/dj-waveform-spec.md`, `docs/deck-spec.md`

## Goal

Replace the scrolling lane’s full-track L0/L1 strip with a bounded **viewport picture ring** so paint cost and GPU memory stay proportional to the window, not track length.

## Decisions

| Topic | Choice |
|-------|--------|
| Scrolling lane content | **Ring only** — no full-track waveform picture |
| Chunk resolution | Hi-res via `LibraryTransport.getWaveformWindow` per chunk |
| Seek / load / resize / zoom / display-mode | Tear down and rebuild all 8 chunks centered on playhead |
| Overlays (grid, cues, loops) | Rebuild for the same ring time span whenever the ring slides or rebuilds |
| Overview strip | Unchanged (full-track overview on deck panel) |
| Host API | No change |

## Geometry

- `chunkPx = floor(windowWidth / 4)` (min 1)
- Visible window ≈ **4** chunks
- Ring holds **8** chunks (= ~2× viewport width)
- Time density: display `pxPerMs` from existing strip/tempo zoom (`stripDisplayPxPerMs` / `cropVisibleMs`)
- `chunkMs = visibleMs / 4` (with `visibleMs` from the viewport + density)
- Playhead-centered: ideal `originMs = positionMs - 4 * chunkMs`, then clamp to track bounds
- Slide rule: when playhead advances (or retreats) by **2 × chunkMs** relative to the ring’s last slide/build anchor, drop those **2** trailing chunks, fetch/append **2** on the leading edge, update `originMs`, re-composite

## Paint model

1. Record up to 8 chunk `Picture`s (each `chunkPx × kWaveformStripHeight`, one window fetch, `buckets ≈ chunkPx`).
2. Record one **composite** `Picture` that draws all chunk pictures side by side.
3. Record overlay `Picture`(s) for the same `originMs` / `spanMs` / `widthPx`.
4. Each frame: translate the composites so the playhead stays viewport-centered (`playheadDx`); single `drawPicture` for waveform + overlays.

Failed window fetches leave that chunk blank (background only); do not toast on the hot path.

## Components

| Unit | Responsibility |
|------|----------------|
| `layout.dart` | Ring constants + pure math (chunk size, centered range, slide detect, clamp) |
| `waveform_ring.dart` | `WaveformRing` state + `WaveformRingNotifier` (fetch, slide, composite, dispose) |
| `waveform_picture.dart` | Existing bar painter + `recordCompositePicture` |
| `scrolling_lane.dart` | Pass viewport width; consume ring; translate by ring origin |
| Overlay providers / geometry | Origin-relative mapping for ring span (overview keeps full-track) |
| `waveform_strip.dart` | Remove once unused |

## Edge cases

- Track ends: clamp ring; if fewer than 8 chunks of time remain, hold shorter ring and do not slide off the end.
- Scrub pointer-up / snap seek: full rebuild (same as seek).
- Empty track / unload: dispose ring and overlays.

## Testing

Pure math tests for chunk sizing, centered range + clamp, slide trigger/origin shift, seek-vs-slide. Existing overlay geometry tests updated for origin-relative mapping. Widget smoke: lane still renders when a ring is provided (extend existing overrides if needed).

## Out of scope

Overview strip redesign, Rust peak encoding, EQ-reactive waveform colors, changing `getWaveformWindow` API.
