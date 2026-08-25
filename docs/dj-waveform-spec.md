# Tech Spec — DJ Software Waveforms

Reference: main engine spec [`tech-spec.md`](tech-spec.md), analyzer spec [`audio-analyzer-spec.md`](audio-analyzer-spec.md).

This document synthesizes how professional DJ applications build, store, and render waveforms, with **Mixxx** as the primary open-source reference (GPLv2, source-verifiable). Closed-source products (Rekordbox, Serato, Traktor) are described where public documentation and community reverse-engineering exist.

---

## Table of Contents

- [1 — Summary](#1--summary)
- [2 — What DJ Waveforms Are For](#2--what-dj-waveforms-are-for)
- [3 — Peak vs RMS (Correcting a Common Assumption)](#3--peak-vs-rms-correcting-a-common-assumption)
- [4 — Industry Comparison](#4--industry-comparison)
- [5 — Mixxx Reference Implementation](#5--mixxx-reference-implementation)
- [6 — Closed-Source Products (Rekordbox, Serato)](#6--closed-source-products-rekordbox-serato)
- [7 — EQ and Waveform Display](#7--eq-and-waveform-display)
- [8 — Rendering Architecture](#8--rendering-architecture)
  - [8.4 Progressive resolution](#84-progressive-resolution-agreed-design)
  - [8.5 Peak buffers + Flutter paint](#85-peak-buffers-rust--host-paint-flutter-decided)
  - [8.6 EQ-aware rendering (future)](#86-eq-aware-rendering-future--architecture-now-wiring-later)
- [9 — Data Model & Storage](#9--data-model--storage)
  - [9.1 Size math](#91-size-math-order-of-magnitude)
  - [9.3 Storage tiers](#93-storage-tiers-agreed-for-mixar)
  - [9.4 Database layout](#94-database-layout-decided)
- [10 — Mixar Today](#10--mixar-today)
- [11 — Recommended Direction for This Project](#11--recommended-direction-for-this-project)
- [12 — Decision log](#12--decision-log)
- [13 — Deferred to implementation](#13--deferred-to-implementation)
- [14 — References](#14--references)

---

## 1 — Summary

| Topic | Finding |
|-------|---------|
| **RMS everywhere?** | **No.** The best-documented open-source DJ app (Mixxx) defaults to **peak (max absolute amplitude)** per visual sample. RMS was a long-standing TODO and became an **optional** analysis mode in 2025 ([mixxx#14325](https://github.com/mixxxdj/mixxx/pull/14325)). |
| **Spectral color** | Nearly universal: offline 3-band split (low / mid / high) drives color. Crossovers and filter types vary. |
| **Scrolling view** | Fixed center playhead; waveform data scrolls horizontally. Zoom defines seconds visible. |
| **Two resolutions** | Detailed waveform for scrolling + low-res **overview** for full-track navigation. |
| **EQ affects display?** | **MVP:** static. **Future:** Mixxx-style band gains at render time (§8.6). |
| **When computed** | Overview at library import; scroll **window** detail at deck load / seek (background). |
| **Mixar storage** | **Overview only** in `library.db` (`track_waveform` table); hi-res window in memory; progressive UI (§8.4). |

---

## 2 — What DJ Waveforms Are For

DJ waveforms are **navigation and beat-matching instruments**, not faithful oscilloscope views.

DJs use them to:

1. **See song structure** — drops, breakdowns, intros, outros, silence.
2. **Align transients** — kick/snare alignment between two decks (stacked scrolling view).
3. **Estimate frequency balance** — via color (RGB / 3-band modes), not for mastering.
4. **Place cues and loops** — beat grid, hot cues, intro/outro markers overlaid on the same view.

They are **not** intended to reflect post-EQ or post-filter audio with perfect accuracy (even where software optionally ties EQ knobs to colors).

---

## 3 — Peak vs RMS (Correcting a Common Assumption)

### 3.1 Definitions

| Method | Per-bucket computation | Visual character |
|--------|------------------------|------------------|
| **Peak** | `max(abs(sample))` over the bucket (per band) | Sharp transients; kicks and snares pop; more “spiky”. |
| **RMS** | `sqrt(mean(sample²))` over the bucket (per band) | Smoother envelope; closer to perceived **energy**; sections easier to read. |
| **Average of peaks** | Mean of peak values from finer strides aggregated into a coarser bucket | Smoother than peak, but **not** true RMS. |

### 3.2 What Mixxx actually does (verified in source)

**File:** `src/analyzer/analyzerwaveform.cpp`

1. **Main (scrolling) waveform** — **peak sampling**:
   - For each audio sample, band-split signal is passed through Bessel IIR filters.
   - `storeIfGreater()` keeps the **maximum** `abs()` value seen within each visual stride.
   - Comment in source explicitly says: *“Take max value, not average of data”*.
   - Commented-out code shows an abandoned experiment with `sample²` accumulation (RMS-like).

2. **Overview / summary waveform** — **average of peak strides**:
   - `WaveformStride::averageStore()` divides accumulated peak strides by a divisor.
   - This is **not** RMS; it is the mean of per-stride peak values over a longer window.

3. **RMS mode (2025+)** — optional:
   - [PR #14325](https://github.com/mixxxdj/mixxx/pull/14325) adds user-selectable RMS analysis.
   - Protobuf schema already had a field for it; implementation was missing for ~13 years.
   - Maintainer notes: RMS helps electronic music **section identification** in overview; some users prefer peak for spotting features.
   - Regenerates waveform data when the preference changes.

### 3.3 Practical takeaway

- **Peak is the historical DJ default** in the only open codebase we can audit.
- **RMS is a desirable option**, especially for overviews and long-track structure, but it is **not** universal.
- Claiming “all DJ software uses RMS” is **not supported** by available evidence; many visuals behave like peak or peak-averaged data.

---

## 4 — Industry Comparison

| Product | Analysis | Amplitude metric | Color modes | EQ → display |
|---------|----------|------------------|-------------|--------------|
| **Mixxx** | Offline at import | Peak (default); RMS optional | Simple, Filtered, RGB, RGB L/R, HSV, Stem | Optional real-time band gain |
| **Rekordbox** | Offline (`rekordbox` analysis) | Not public; behaves like peak/RGB | Blue (amplitude), RGB, 3-Band | Static (no knob feedback) |
| **Serato** | Offline | Not public | Frequency-colored (low → red) | Static |
| **Traktor** | Offline | Not public | Blue/orange or RGB depending on version | Static |
| **Mixar** | JIT on deck load | Peak (max abs) | RGB-style spectral | Not implemented |

---

## 5 — Mixxx Reference Implementation

### 5.1 Analysis pipeline

```text
Audio file (import)
    → decode to stereo float
    → 3× Bessel 4th-order IIR band filters (per channel)
         Low:  below 600 Hz
         Mid:  600 Hz – 4000 Hz
         High: above 4000 Hz
    → for each visual stride:
         peak_L_band = max(abs(filtered_L)) across stride
         peak_R_band = max(abs(filtered_R)) across stride
    → quantize to uint8 (0–255) per band per channel
    → persist to library DB (AnalysisDAO)
```

**Visual sample rate:** `441` visual samples per second of audio (`mainWaveformSampleRate` constant).  
At 44.1 kHz audio, each visual sample spans ~100 audio samples.

**Overview resolution:** `2 × 1920` visual samples (~3840 points), sized for a full-width HD overview.

### 5.2 Stored data shape

**File:** `src/waveform/waveform.h`

```cpp
struct WaveformFilteredData {
    unsigned char low;
    unsigned char mid;
    unsigned char high;
    unsigned char all;   // full-band peak
};

struct WaveformData {
    WaveformFilteredData filtered;
    unsigned char stems[kMaxSupportedStems];
};
```

- **Per visual sample:** L and R each store 4 amplitudes (all, low, mid, high).
- **8-bit** values; normalized at analysis time with `× 255` scaling.

### 5.3 Waveform display types

Documented in [mixxx/manual#603](https://github.com/mixxxdj/manual/issues/603):

| Type | Behavior |
|------|----------|
| **Simple** | Monochrome full-band amplitude. |
| **Filtered** | Separate colored layers per band (low bottom, high top). |
| **RGB** | Bar height from full signal; color from weighted low/mid/high amplitudes. |
| **RGB L/R** | RGB computed independently per channel. |
| **HSV** | Low → brightness, high → saturation; hue fixed. |
| **Stem** | Per-stem layers when stem files are loaded. |

Since Mixxx 2.4, scrolling renderers use **GLSL shaders** at 60 fps ([Mixxx blog 2024](https://mixxx.org/news/2024-02-23-improved-waveforms/)).

### 5.4 Scrolling view mechanics

- **Playhead** fixed at horizontal center.
- Renderer maps pixel columns to a range of visual frames using `firstDisplayedPosition` / `lastDisplayedPosition`.
- **Zoom** changes seconds-per-pixel (visual increment per pixel).
- Layers drawn on top: beat grid, loops, cues, intro/outro, end-of-track.
- **VSync / PLL** optional for phase-locked smooth scrolling.

---

## 6 — Closed-Source Products (Rekordbox, Serato)

Public docs and DJ community sources (not source code):

### Rekordbox

- **Blue mode** — classic amplitude waveform (peaks/valleys, loud vs quiet).
- **RGB mode** — red ≈ lows, yellow/green ≈ mids, blue ≈ highs ([Hot Cue DJ overview](https://reallychrism.substack.com/p/the-secret-language-of-waveforms)).
- **3-Band mode** — separate frequency components (CDJ-3000); lows blue, mids amber, highs white.
- Analysis runs in Rekordbox on import; CDJs read precomputed data from USB/network.
- [Schematic Sound](https://schematicsound.com/2025/06/20/dj-waveform-colours/) notes Rekordbox may **normalize color balance** so structure remains visible on bass-heavy tracks — colors are a **guide**, not a spectrum analyzer.

### Serato

- [Serato support docs](https://support.serato.com/hc/en-us/articles/360001462076): main waveform is a **centered snapshot**; colors reflect **dominant frequencies** (red = low, lighter = high).
- Separate **overview** strip for full-track navigation.
- Cues shown as colored flags; zoom via +/- or scroll wheel.

### What we cannot verify without reverse engineering

- Exact filter topology and crossover frequencies.
- Whether scrolling data uses peak, RMS, or hybrid.
- Internal bit depth and downsampling strategy.

---

## 7 — EQ and Waveform Display

This is the most implementation-specific topic and **differs by product**.

### 7.1 Mixxx — EQ **does** affect the waveform (selectively)

**Files:** `src/waveform/renderers/waveformrenderersignalbase.cpp`, `waveformrendererfilteredsignal.cpp`, `waveformrendererrgb.cpp`

Mechanism:

1. Offline analysis produces **fixed** low/mid/high amplitudes per visual sample (using analysis filters at 600 Hz / 4000 Hz).
2. At **render time**, `getGains()` reads deck EQ knob values from control objects:
   - `[EqualizerRack1_DeckN_Effect1] parameter1/2/3` → low/mid/high **visual gain multipliers**.
   - Kill buttons zero a band’s layer.
3. Gated by `[Channel] filterWaveformEnable` — can be turned off.
4. [Issue #14901](https://github.com/mixxxdj/mixxx/issues/14901) / [PR #14998](https://github.com/mixxxdj/mixxx/pull/14998): users requested **disable EQ influence** because cutting lows makes drops hard to see; preference added.

Important nuances (from Mixxx developers):

- Visualization EQ is a **1:1 mapping** to the three **analysis bands**, not a re-run of the audio EQ’s biquad/shelf curves.
- Changing audio EQ crossover in preferences does **not** change analysis bands — they can mismatch.
- **Filter** (quick effect) does **not** affect the waveform — only the 3-band EQ rack.
- **Simple** waveform type ignores EQ for display.

### 7.2 Rekordbox / Serato — EQ **does not** affect the waveform

Community and Mixxx developer consensus ([mixxx#14901](https://github.com/mixxxdj/mixxx/issues/14901)):

- Colors are from **one-time import analysis**.
- Turning hardware or software EQ during performance changes **audio**, not the precomputed colors.
- This makes structural landmarks (drops, breakdowns) stable while mixing.

### 7.3 Design implications

| Approach | Pros | Cons |
|----------|------|------|
| **Static analysis (Serato/Rekordbox)** | Stable landmarks; predictable mixing | Display diverges from heard audio after heavy EQ |
| **EQ-scaled bands (Mixxx RGB)** | Display hints at current tonal balance | Can hide drops/cues when lows are cut; confusing if filter doesn’t affect display |
| **Hybrid** | Static structure + subtle EQ tint | More complex; needs clear UX |

**MVP:** **Static** waveform (colors from analysis only). **Post-MVP:** Mixxx-style EQ band scaling at **render time** (multiply low/mid/high by deck EQ knob values). Architecture must support this from day one (§8.6) even though MVP does not wire EQ knobs to the renderer.

---

## 8 — Rendering Architecture

### 8.1 Common pattern (scrolling main waveform)

```text
┌─────────────────────────────────────────────────────────────┐
│  Deck A lane  │░░░░▓▓▓███▓▓│← spectral bars scroll ←──────│
├───────────────┼─────────────┼───────────────────────────────┤
│  Deck B lane  │──────▓▓▓████│                               │
│               │      ▲      │                               │
│               │   playhead  │  (fixed at center)           │
└─────────────────────────────────────────────────────────────┘
```

1. **Input:** precomputed visual samples + playhead position (seconds or frames).
2. **Window:** `visible_duration` (e.g. 8–30 s); map each pixel column → time → visual sample index.
3. **Interpolate** between visual samples when zoomed in.
4. **Color:** function of `(low, mid, high)` amplitudes at that column.
5. **Overlays:** beat grid (from beat grid analysis), cues, loop regions — separate render passes.
6. **Update rate:** 60 fps during playback; position from engine (may interpolate between status polls).

### 8.2 Overview waveform

- Entire track in ~2000–4000 samples.
- Click to seek; shows cue markers.
- Often uses **coarser** aggregation (Mixxx: average of peak strides).

### 8.3 Performance

- **Never** decode audio or run filters in the render loop.
- **Peaks in Rust, paint in Flutter** (§8.5): library builds overview / window spectral peak buffers; the Flutter host paints lanes (`CustomPainter`), not a separate JS Canvas path.
- Beat grid lines: derive from library `beat_grid` (BPM + first beat offset), not from waveform data.

### 8.5 Peak buffers (Rust) + host paint (Flutter) (decided)

Waveform **analysis** (spectral peaks) runs in Rust (`library` / `audio-core`). **Rasterization for display** stays in the Flutter host (see `apps/gui-flutter` README). The UI is a thin consumer of peak arrays:

```text
Rust (library / host-flutter FRB)
  ├─ load overview / window peaks from library or JIT
  ├─ apply display gains (future: deck EQ → band multipliers, §8.6)
  └─ expose peaks to UI: LibraryTransport.get_waveform_overview / get_waveform_window

Frontend (Flutter)
  └─ paint lanes per deck (CustomPainter); handle zoom/seek input only
```

MVP ships peak arrays (overview + window detail) over FRB; Flutter paints bars each frame (or dirty-region updates).

Library access: `Library::get_track_waveform` / window APIs in Rust; Flutter FRB `LibraryTransport` wraps them — not a separate “Dart fetches PCM and analyzes” architecture.

### 8.6 EQ-aware rendering (future — architecture now, wiring later)

Mirror Mixxx `getGains()`: stored peaks are **immutable**; EQ affects **display multipliers only**.

```text
struct WaveformDisplayGains {
    low: f32,   // from deck EQ low knob (future)
    mid: f32,
    high: f32,
    // optional: filterWaveformEnable bool
}

fn color_and_height(
    peak: SpectralPeak,  // from DB / window analysis — never re-filtered audio
    gains: WaveformDisplayGains,
) -> (height, rgb) {
    let low = peak.low * gains.low;
    let mid = peak.mid * gains.mid;
    let high = peak.high * gains.high;
    // ...
}
```

MVP: `gains` always `1.0`. Renderer and analysis APIs accept `WaveformDisplayGains` (or `Option`) so EQ can be plugged in without changing the `track_waveform` schema or re-analysis.

### 8.4 Progressive resolution (agreed design)

**Persisted data:** overview only (full track, ~2k–4k samples) in the library DB.

**Scrolling view:** the **visible** time window is what the user sees; **hi-res analysis** covers visible **plus buffer ahead and behind** so playback and small seeks do not wait on a new decode pass.

```text
        [  buffer behind  ][   visible window   ][  buffer ahead  ]
        |←── L2 / detail ──→|←── L1 priority ──→|←── L2 / detail ──→|
                              ▲ playhead (center)
```

```text
Track load / seek
    │
    ├─► [instant] L0: render visible range from DB overview
    │
    ├─► [background] L1: hi-res buckets for visible window (~1 bucket / pixel)
    │
    └─► [background] L2: hi-res buckets for buffer regions before & after visible
```

| Stage | Source | Latency | Use |
|-------|--------|---------|-----|
| **L0 — Overview** | DB overview, sliced to visible range | Immediate | First paint on deck load / seek |
| **L1 — Visible detail** | JIT analysis of visible `[t₀, t₁]` | ~100 ms–1 s | Hi-res in the lane center |
| **L2 — Buffer detail** | JIT analysis of **before** and **after** visible range | Background | Scroll / nudge without stall |

**On playhead move / seek:** L0 immediately; **cancel** in-flight jobs; restart L1 (visible) then L2 (buffers).

**Zoom:** user-adjustable visible duration; buffer margins scale with zoom (config); re-run L1/L2 for new ranges.

**Terminology — “buckets” (O11):** one **bucket** = one visual sample: aggregated `(low, mid, high)` for a short audio slice. **Overview:** fixed bucket count for the full track. **Scrolling:** ~**one bucket per horizontal pixel** over the analyzed region. L1 targets the **visible** width; L2 uses the **same bucket density** for **buffer ahead and behind** (not stored in DB).

**UI feedback:** subtle loading state on the lane (e.g. faint pulse or “soft” bars) until L1 arrives — driven by resolution tier rather than missing data.

**Why this works:**

- DB stays small (overview only; see §9).
- Deck feels responsive (overview is already on disk after `analyze_track`).
- CPU cost scales with **visible + buffered seconds**, not track length.
- No mandatory filesystem scroll cache; optional in-memory reuse of the last L1 window per deck is enough for MVP.

**Renderer contract:**

```text
drawScrollingLane(
  overview: &[SpectralPeak],
  detail: Option<&[SpectralPeak]>,       // visible + buffered spans merged
  detail_range: (i32, i32),             // [start_ms, end_ms] detail covers
  visible_range: (i32, i32),              // subset currently on screen
  position_ms, duration_ms,
)
```

Flutter paint (§8.5) maps overview for gaps; prefers `detail` buckets where available.

---

## 9 — Data Model & Storage

Waveform data is **large at scale**. A naive “one BLOB per track in the main library DB” design can grow to **many gigabytes** for typical DJ libraries (5k–20k tracks). Storage must be a first-class design constraint, not an afterthought.

### 9.1 Size math (order-of-magnitude)

Two common encoding strategies behave very differently:

#### A — Fixed visual rate (Mixxx-style: 441 samples / second of audio)

Bytes per track ≈ `(duration_ms / 1000) × visual_rate × bytes_per_sample`.

| Encoding | Bytes / visual sample | 7 min track | 10,000 tracks (7 min avg) |
|----------|----------------------|-------------|---------------------------|
| RGB mono (`low,mid,high` uint8) | 3 | **~555 KB** | **~5.3 GB** |
| RGB stereo (L/R × 3 bands uint8) | 6 | **~1.1 MB** | **~10.5 GB** |
| Mixxx full (L/R × 4 bands uint8) | 8 | **~1.5 MB** | **~14 GB** |
| Overview only (3840 samples × 3) | — | **~11 KB** | **~110 MB** |

A single **60 min** mix at 441/s mono RGB is **~4.8 MB** of scroll data alone.

#### B — Fixed bucket cap (Mixar today: max 16,384 buckets)

Bytes per track ≈ `cap × bands × sizeof(sample)` — **bounded regardless of track length**.

| Encoding | 16,384 buckets | 10,000 tracks |
|----------|----------------|---------------|
| 3 × `f32` (in-memory today) | ~192 KB / track | **~1.9 GB** |
| 3 × `uint8` (recommended if capped) | **~48 KB** / track | **~480 MB** |
| + zstd compression (typical) | ~20–35 KB / track | **~200–350 MB** |

Long tracks **lose temporal resolution** with a fixed cap (a 60 min track still gets 16k buckets ≈ 220 ms/bucket). That may be acceptable for overview-style navigation but is weak for zoomed beat-matching unless detail is regenerated or stored separately.

#### SQLite overhead

BLOB pages, indexes, and fragmentation add **~10–30%** on top of raw payload. Storing multi-MB blobs inline also increases **VACUUM / backup** cost and keeps the hot library DB on disk longer during analysis writes.

### 9.2 What Mixxx does (reference)

Mixxx stores waveform **analysis blobs in the library database** (via `AnalysisDAO`), at **441 visual samples/sec** with uint8 bands — so size scales with duration. Rendered overview **pixmaps** are cached separately in memory (`OverviewCache` / `QPixmapCache`), not re-read from raw bytes every paint.

Takeaway: even Mixxx separates **persistent analysis bytes** from **cheap display cache**, and accepts per-track size scaling with duration.

### 9.3 Storage tiers (agreed for Mixar)

**Decision:** persist **overview only** in the library DB. Scroll detail is **not** stored on disk — computed per visible window at runtime (§8.4).

```text
┌─────────────────────────────────────────────────────────────────┐
│ Tier 0 — Metadata (main library DB)                             │
│   track_id, waveform_version, duration, content_hash            │
├─────────────────────────────────────────────────────────────────┤
│ Tier 1 — Overview BLOB (main library DB, on analyze/import)   │
│   ~2k–4k uint8 RGB samples, full track                          │
│   ~12–48 KB/track → 10k tracks ≈ 120–480 MB                     │
├─────────────────────────────────────────────────────────────────┤
│ Tier 2 — Window detail (memory only, on deck load / seek)      │
│   Hi-res spectral peaks for visible time range only             │
│   Discarded on unload; optional short-lived cache per deck      │
└─────────────────────────────────────────────────────────────────┘
```

| When | What happens |
|------|----------------|
| `analyze_track` / import | Write Tier 0 + Tier 1 overview to DB |
| Load track to deck | Read overview → **instant** L0 scroll view (§8.4) |
| Playing / idle on deck | Background: analyze **visible window** → L1 detail |
| Seek | L0 immediately; restart L1 for new window |
| Unload | Drop Tier 2 memory |

Optional later: filesystem LRU for repeated window analysis. **Not required for MVP.**

### 9.4 Database layout (decided)

**Same database** as the rest of the library (`library.db`). Waveform data lives in a **new table** — not columns on `tracks` or `track_analysis`, and not a separate database file.

```sql
CREATE TABLE IF NOT EXISTS track_waveform (
    track_id            TEXT PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    version             INTEGER NOT NULL,
    amplitude_mode      TEXT NOT NULL,  -- 'peak' | 'rms' (mvp: peak)
    channel_mode        TEXT NOT NULL,  -- 'mono' | 'stereo' (config, §D13)
    overview_count      INTEGER NOT NULL,  -- fixed constant, §D7
    overview_bytes      BLOB NOT NULL,  -- zstd(uint8[overview_count × bytes_per_sample])
    generated_at        TEXT NOT NULL
);
```

| Rule | Value |
|------|-------|
| Max BLOB size | 64 KB per row (overview only) |
| Scroll detail | **Not stored** — runtime window analysis (§8.4) |
| Cascade | `ON DELETE CASCADE` when track removed |

`overview_rate` is derivable as `overview_count / (duration_ms / 1000)` and need not be stored unless we want to avoid joining `tracks` for display.

Rejected alternatives: separate `library_waveforms.db`, filesystem sidecar files, scroll blobs in SQLite.

### 9.5 Mitigations checklist

| Technique | Effect |
|-----------|--------|
| **uint8 quantization** | 4× smaller vs `f32` |
| **Mono RGB (3 bytes/sample)** | 2× smaller vs stereo bands; sufficient for stacked deck UI |
| **Overview-only in DB** | ~100× smaller than full-rate scroll data at library scale |
| **Window-only JIT analysis** | CPU ∝ visible seconds, not track length |
| **Content-hash dedup** | Same file in multiple playlists → one overview row |
| **zstd on BLOBs** | Often 2–3× on spectral uint8 data |
| **Lazy window detail** | No scroll persistence; regenerate on seek (fast for ~24 s) |
| **Version field + rebuild** | Safe algorithm upgrades without silent bloat |

### 9.6 Encoding format (compact)

Payload inside `overview_bytes` is **zstd-compressed**. Decompressed layout:

```text
sample_count × (low u8, mid u8, high u8)   -- mono RGB
-- or sample_count × (L_low, L_mid, L_high, R_low, R_mid, R_high) when channel_mode = stereo
```

`version` + `amplitude_mode` + `channel_mode` live in columns; no custom `WAVE` file header required inside the BLOB.

### 9.6.1 Modular band filters (analysis)

Band splitting is **pluggable** so crossovers and filter types can be experimented with (Bessel, Butterworth, etc.):

```text
trait WaveformBandFilter {
    fn process_frame(&mut self, mono: f32) -> SpectralBands { ... }
}
```

Analysis pipeline selects implementation from config / `WaveformAnalysisConfig`. MVP ships one default (e.g. Butterworth or Bessel at 600 / 4000 Hz); others added without changing storage format.

### 9.7 Comparison to current Mixar

| Aspect | Today | Target |
|--------|-------|--------|
| Scroll data | Full-track JIT (`get_track_waveform`) | **Window-only** JIT + overview slice (§8.4) |
| Persistence | In-memory cache only | **Overview in DB** on `analyze_track` |
| First paint | Wait for full-track analysis | **Instant** from overview |
| Resolution | Single tier (16k buckets full track) | **L0 overview → L1 window detail** |
| Library DB growth | N/A | ~12–48 KB / track (bounded) |

**Next implementation steps:**

1. Persist overview during `analyze_track`; if missing on **first deck load**, call `analyze_track` then read `track_waveform`.
2. Rust: `Library::get_track_waveform(track_id)`; `analyze_waveform_window(track_id, start, duration)` for visible + buffer spans; shared decode cache.
3. Flutter paints overview + detail peaks; progressive L0 → L1 → L2 (ahead + behind).

### 9.8 Generation parameters (decided defaults)

| Parameter | MVP value | Notes |
|-----------|-----------|-------|
| Overview buckets | **Fixed** `OVERVIEW_SAMPLE_COUNT` (config; tune to max overview UI width, e.g. 2048–3840) | Full track always maps to this many samples |
| Window buckets (L1) | ~1 per pixel × **visible** width | Priority hi-res |
| Buffer buckets (L2) | ~1 per pixel × **buffer ahead + behind** | Same density as L1 |
| Amplitude | **Peak** | `amplitude_mode` column; RMS later as user preference |
| Channel layout | **Config** `mono` \| `stereo` | `channel_mode` column |
| BLOB | **zstd** compressed | Decompress on read |
| Filter | Modular trait; one default impl | Butterworth / Bessel selectable in config |
| Colors | **Rekordbox RGB** | Red low, green mid, blue high |
| EQ display | Static gains (= 1.0) | Future: Mixxx-style band multipliers (§8.6) |

### 9.9 Versioning

Bump `version` when changing filters, crossovers, or amplitude mode. Invalid overview → re-analyze on next `analyze_track`. Window detail is never versioned on disk — always derived from current algorithm + source file.

---

## 10 — Mixar Today

**Location:** `audio-core` / library waveform APIs; Flutter paint in `apps/gui-flutter/lib/mixer/waveform/`.

| Aspect | Current implementation |
|--------|------------------------|
| Amplitude | **Peak** (`max(abs)`) per bucket per band |
| Filters | One-pole IIR (~250 Hz / ~4000 Hz) — simpler than Mixxx |
| Bands | low / mid / high `SpectralPeak` |
| When computed | JIT via library overview / window APIs on deck load |
| Cache | In-memory per path + bucket count |
| Resolution | Adaptive 4096–16384 buckets (~13 ms/bucket) |
| Display | Dual-lane Flutter `CustomPainter`, center playhead, 24 s window |
| EQ link | **None** — static colors |
| Overview strip | Implemented in Flutter (`overview_strip.dart`) |
| Beat grid overlay | Implemented when analysis present |
| RMS option | Not implemented |

---

## 11 — Recommended Direction for This Project

### Phase A — Analysis & persistence

1. **Overview in DB** on `analyze_track` (~3840 samples, uint8 RGB).
2. **Window-only** high-res analysis for the visible scroll range (§8.4).
3. **Replace one-pole filters** with Bessel 4th at **600 Hz / 4000 Hz** crossovers.
4. **Add RMS mode** for overview (optional peak for window detail).

### Phase B — Display

1. **Progressive lanes:** overview slice first, refine when window job completes.
2. **Beat grid** from library beat grid analysis.
3. **Cue markers** when cue system exists.
4. **60 fps** scroll; restart window job on seek.

### Phase C — EQ & overlays

1. **MVP:** static waveform; `WaveformDisplayGains` wired to identity (§8.6).
2. **Post-MVP:** Mixxx-style EQ band scaling at render time (no re-analysis).
3. **Beat grid overlay** on scrolling lanes (live, from library beat grid).

### Phase D — Not recommended for MVP

- Real-time FFT in the render path.
- Tying filter knob to waveform (inconsistent with Mixxx itself).
- Claiming colors are mastering-accurate frequency balance.

---

## 12 — Decision log

| # | Topic | Decision |
|---|--------|----------|
| D1 | **What to persist** | Overview only (fixed bucket count per track) |
| D2 | **Scroll detail** | Window-only JIT in memory; not on disk |
| D3 | **Progressive UI** | L0 overview → L1 window detail → L2 ahead/behind buffers |
| D4 | **Database** | Same `library.db`; `track_waveform` table |
| D5 | **EQ → waveform MVP** | Static (gains = 1.0) |
| D6 | **EQ → waveform future** | Mixxx-style band multipliers at render time; architecture in §8.6 |
| D7 | **Overview size (O3)** | **Fixed** `OVERVIEW_SAMPLE_COUNT` (config constant; not duration-scaled) |
| D8 | **Amplitude (O1, O2)** | **Peak** for MVP; `amplitude_mode` + future user pref for RMS |
| D9 | **Band filters (O4)** | **Modular** trait; Butterworth / Bessel etc. via config |
| D10 | **BLOB (O5)** | **zstd-compressed** `overview_bytes` |
| D11 | **Channels (O6)** | **Config** `mono` \| `stereo`; stored in `channel_mode` |
| D12 | **Generation (O7)** | `analyze_track` on import; **also on first deck load** if row missing |
| D13 | **Stale data (O8)** | Deferred past MVP |
| D14 | **Dedup (O9)** | One row per `track_id` |
| D15 | **Zoom (O10)** | User zoom; L2 chunks **before and after** visible window |
| D16 | **Window resolution (O11)** | ~1 bucket/pixel for **visible**; L2 buffers **ahead + behind** at same density |
| D17 | **L1 transition (O12)** | Resolve in prototype (hard swap vs crossfade) |
| D18 | **L2 (O13)** | **Yes** — analyze ahead and behind playhead |
| D19 | **Seek (O14)** | **Cancel** in-flight window jobs immediately |
| D20 | **Colors (O15)** | **Rekordbox RGB** (red / green / blue) |
| D21 | **Beat grid (O17)** | **Live overlay** on scrolling lanes |
| D22 | **Library UI (O18)** | **Decks only** for now |
| D23 | **Rendering (O19)** | **Confirmed:** Rust library peak buffers; Flutter paints lanes (`CustomPainter`) |
| D24 | **Window API (O20)** | By **`track_id`**; reuse decode cache |

---

## 13 — Deferred to implementation

All product decisions are locked (§12). Remaining items are **engineering choices** during build:

| Item | Approach |
|------|----------|
| L1 transition (O12) | Prototype hard swap vs crossfade; pick what looks better |
| `OVERVIEW_SAMPLE_COUNT` | Set when lane/overview UI width is fixed (likely 2048–3840) |
| Default band filter | Bessel vs Butterworth at impl time; modular trait supports both |
| Buffer margin | Config ratio or seconds each side of visible window (e.g. 50% of visible width) |
| Stale overview | Post-MVP (D13) |

---

## 14 — References

### Mixxx (primary open-source reference)

| Resource | URL |
|----------|-----|
| Waveform analyzer | https://github.com/mixxxdj/mixxx/blob/main/src/analyzer/analyzerwaveform.cpp |
| Waveform stride (peak + average store) | https://github.com/mixxxdj/mixxx/blob/main/src/analyzer/analyzerwaveform.h |
| Waveform data types | https://github.com/mixxxdj/mixxx/blob/main/src/waveform/waveform.h |
| EQ gain on render | https://github.com/mixxxdj/mixxx/blob/main/src/waveform/renderers/waveformrenderersignalbase.cpp |
| Filtered renderer + kill | https://github.com/mixxxdj/mixxx/blob/main/src/waveform/renderers/waveformrendererfilteredsignal.cpp |
| RMS analysis PR | https://github.com/mixxxdj/mixxx/pull/14325 |
| Disable EQ on waveform PR | https://github.com/mixxxdj/mixxx/pull/14998 |
| EQ waveform issue | https://github.com/mixxxdj/mixxx/issues/14901 |
| Waveform types explanation | https://github.com/mixxxdj/manual/issues/603 |
| Scrolling waveform 2.4 blog | https://mixxx.org/news/2024-02-23-improved-waveforms/ |

### Closed-source / community

| Resource | URL |
|----------|-----|
| Serato deck waveform area | https://support.serato.com/hc/en-us/articles/360001462076 |
| Rekordbox waveform modes (community) | https://reallychrism.substack.com/p/the-secret-language-of-waveforms |
| Rekordbox color accuracy (community) | https://schematicsound.com/2025/06/20/dj-waveform-colours/ |

### This repo

| Resource | Path |
|----------|------|
| Current peak generator | `audio-core` / library waveform APIs |
| GUI scrolling lane | `apps/gui-flutter/lib/mixer/waveform/scrolling_lane.dart` |
| GUI waveform section | `apps/gui-flutter/lib/mixer/waveform_section.dart` |
| Analyzer roadmap (waveforms) | `docs/audio-analyzer-spec.md` §2, §14 |
