# Flutter waveform rendering (Impeller picture + transform)

Date: 2026-08-14  
Status: accepted (user waived remaining review gates until PR)  
Depends: `2026-08-13-flutter-engine-transport-design.md`, `2026-08-13-flutter-library-transport-parity-design.md`, `2026-08-14-flutter-track-dnd-design.md`  
Reference: `docs/dj-waveform-spec.md` §8.4, `docs/deck-spec.md` §5.2, Tauri `waveform_render.rs` (color/math only)

## Goal

Wire dual-deck waveforms in Flutter: L0 overview from `library.db`, lazy L1 window detail, beat grid, overview strips (waveform stack + deck panel), click-to-seek and drag-scrub. Rust analyzes; Flutter paints. Do **not** port Tauri `render_waveform_lane` packed RGBA frames.

## Decisions

| Topic | Choice |
|--------|--------|
| Rasterization | Flutter `ui.Picture` + layer `Transform`; rebuild picture on data/size/L1, not on playhead ticks |
| Data from Rust | Packed RGB `uint8` peaks (`peaks_to_rgb_bytes`, mono, 3 bytes/sample) |
| L0 | `getWaveformOverview(trackId)` from DB (load already `ensure_track_waveform`) |
| L1 | One JIT window: visible + 1 viewport buffer each side; `compute_spectral_window` on decode cache |
| Visible span | 24_000 ms at 1×, scaled by playback ratio (`normToSpeedRatio`), clamped 0.5–2 |
| Overview placement | Both: per-deck strip in waveform stack **and** deck-panel preview; shared L0 cache |
| Seek | Overview click; scrolling lane click + center-playhead drag-scrub |
| Beat grid | Overlay, not baked into the picture; skip if no analysis |
| EQ → waveform | Unity gains (static colors). Architecture: peaks immutable |
| Zoom | Out of scope (P1) |
| Cues / loops on waveform | Out of scope (P1) |

## Architecture

```text
LibraryTransport
  getWaveformOverview(trackId) → WaveformPeaks?   // L0 RGB bytes
  getWaveformWindow(trackId, startMs, endMs, buckets) → WaveformPeaks
  getBeatGrid(trackId) → BeatGrid?                // beats/downbeats seconds + bpm

EngineTransport
  seek(deckId, positionMs) → CmdBody::Seek

Engine UI
  snapshot: trackId, durationMs, playing, speed, tempoRange  (low-rate)
  playheads notifier: positionMs                             (high-rate, isolated)

Flutter paint
  decode RGB → SpectralPeak[]
  record Picture of bars (L0; L1 where time overlaps)
  Transform scrolls; center playhead is a 1px overlay
  beat grid CustomPaint overlay
```

L1: after L0 paints, request `[position − 1.5·visible, position + 1.5·visible]` with `buckets ≈ 3 × laneWidth`. Generation token cancels stale replies. Restart when playhead reaches 35% of the buffer edge (`WAVEFORM_REFRESH_MARGIN`). L0 interpolated for times outside the detail window.

Playhead interpolation: ticker in the lane widget (Tauri `useSmoothPlayhead`): extrapolate while playing; snap on seek ≥ 180 ms; ignore engine position while scrubbing.

## Components

| File | Role |
|------|------|
| `lib/mixer/waveform/peaks.dart` | RGB decode, `SpectralPeak`, `peakAtTime`, L0/L1 merge |
| `lib/mixer/waveform/spectral_color.dart` | Tauri LOW/MID/HIGH mix + bar alpha |
| `lib/mixer/waveform/layout.dart` | visible ms, buffer origin, overview window rect, center-scrub math |
| `lib/mixer/waveform/beat_grid.dart` | even BPM grid x-positions from snapshot |
| `lib/mixer/waveform/waveform_picture.dart` | record `ui.Picture` of spectral bars |
| `lib/mixer/waveform/scrolling_lane.dart` | sliding picture + transform + scrub + grid overlay |
| `lib/mixer/waveform/overview_strip.dart` | full-track L0 + playhead + visible-window rect + click seek |
| `lib/mixer/waveform_section.dart` | dual deck: overview + lane each; shared center playhead |
| `lib/mixer/engine_ui.dart` | trackId / duration / speed / tempoRange; position **not** on this snapshot |
| `crates/library` | `compute_waveform_window` on decode cache (decode-from-path if miss) |
| `crates/host-flutter` | FRB methods above + `EngineEvt.duration_ms/speed/tempo_range` |

Raise waveform region default height so two overviews + two lanes fit (~160 px; min 110).

Deck panel: thin `OverviewStrip` under the title, above performance/jog.

## Data flow

1. Load → `Updated` with `trackId` + `durationMs` → fetch L0 + beat grid (cached by trackId).
2. Lane size known → record L0 picture for 3× viewport; start L1 fetch.
3. Position/ticker → `dx = width/2 − (pos − originMs) × pxPerMs`.
4. L1 lands (matching token) → rebuild picture.
5. Pointer on overview: `ms = x/width × durationMs` → `seek`.
6. Pointer on lane: center mode `ms = anchorPos − Δx/width × spanMs` → throttled `seek` (~32 ms).

## Error handling

- Missing overview: empty lane, keep placeholder copy when **both** decks unloaded.
- L1 / beat-grid failure: keep L0; do not toast (non-fatal). Log via `FlutterError.reportError` only for unexpected exceptions.
- Seek / load errors: existing destructive toast.
- No `trackId`: skip fetch (should not happen after load; path loads import a library row).

## Testing

- Dart unit: RGB round-trip, spectral color vs Tauri constants, `peakAtTime` prefers L1, visible-ms clamp, overview window percent, center-scrub math, beat-grid bar vs beat, `applyEngineEvt` ignores position for chrome fields.
- Host: overview after prepare; window non-empty after load; seek publishes Updated/Position.
- Widget: empty placeholder; seeded trackId hides empty copy; overview strip present on deck panel.

## Non-goals

- Tauri `pack_waveform_frame` / `render_waveform_lane`
- Fragment shaders
- User zoom
- Cue/loop overlays
- EQ-tinted waveform
- Web
