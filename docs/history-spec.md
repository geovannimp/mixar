# History & Performance Logs

Reference: [`tech-spec.md`](tech-spec.md) §10, [`deck-spec.md`](deck-spec.md) Phase 5, [GitHub issue #52](https://github.com/geovannimp/mixar/issues/52).

This document defines **set history** — an append-only log of tracks played during DJ sessions — including storage, session boundaries, logging rules, and export.

---

## Table of Contents

- [1 — Summary](#1--summary)
- [2 — Competitor Context](#2--competitor-context)
- [3 — Scope & Non-goals](#3--scope--non-goals)
- [4 — Logging Rules](#4--logging-rules)
- [5 — Session Model](#5--session-model)
- [6 — Storage (XSPF)](#6--storage-xspf)
- [7 — Track Metadata (ISRC)](#7--track-metadata-isrc)
- [8 — Data Flow & Ownership](#8--data-flow--ownership)
- [9 — Library & UI](#9--library--ui)
- [10 — Settings](#10--settings)
- [11 — Live Output (OBS)](#11--live-output-obs)
- [12 — Export (Derived Formats)](#12--export-derived-formats)
- [13 — Phased Delivery](#13--phased-delivery)
- [14 — Acceptance Criteria](#14--acceptance-criteria)
- [15 — References](#15--references)

---

## 1 — Summary

Mixar records **what was played, on which deck, when it started, and when it ended** during a performance session. Each session is persisted as a **live-updated XSPF file** under app support storage. Re-playing the same track in one set **appends a new entry**. **Sampler** playback is **never** logged. All **main deck channels** (2 today, **4 in Phase 5**) are logged.

A play is logged only after **minimum effective output** and **minimum play duration** thresholds are met (both configurable). **Effective output** combines deck channel fader and **crossfader** gain. **Preview (PFL)** is ignored.

Session boundaries:

- **Idle split** — when no deck meets the “active output” condition (§4.4) for longer than `history_session_idle_minutes` (default **5**), the session closes.
- **Manual** — user can start a new session or **resume** the previous one while no successor session exists yet (§5.4).

On app restart, if the last session is still inside the idle window, the user is **prompted** to restore it or start fresh.

History is Phase 5 library/workflow polish — adjacent to collections and export, not a deck performance control.

---

## 2 — Competitor Context

| Product | Session split | Storage | Export |
|---------|---------------|---------|--------|
| Serato DJ Pro | Per gig / day | Binary `History/Sessions/*.session` | TXT, CSV, M3U |
| Rekordbox | Per calendar date | Library + USB history playlists | Tab TXT (UTF-16), M3U8 |
| Traktor Pro | Per launch + archive | XML `history_*.nml` | HTML, NML |
| Mixxx | Per app launch | Hidden playlists in SQLite | M3U, CSV, TXT, PLS |
| VirtualDJ | Daily M3U + flat royalty log | `History/*.m3u`, `tracklist.txt` | M3U on disk, CSV via browser |
| Engine DJ | Per drive / library | SQLite `hm.db` | CSV, M3U, JSON |

No competitor uses XSPF as the **live** history store. Mixar chooses XSPF as the canonical on-disk format for openness ([xspf.org](https://www.xspf.org/)) while still offering derived CSV / M3U8 / plain TXT export for interoperability.

---

## 3 — Scope & Non-goals

### In scope (Phase 5)

- Append-only play log for **deck** playback (all deck indices).
- One **XSPF file per session**, updated as plays occur (`started_at` / `ended_at` per entry).
- Volume + duration gates before an entry is committed.
- Idle-based and **manual** session split.
- Restore prompt on app restart (inside idle window).
- Session list + detail in library UI.
- Derived export: CSV, M3U8, plain TXT.
- **ISRC** on `tracks` (tag import; editable in track detail).
- “Save session as playlist” (ordered `Playlist` collection).

### Out of scope (initial)

- Auto-syncing sessions into `CollectionType::History` (reserved in `tech-spec.md` §10 for possible **manual imports** only — **ignored for now**).
- Import Serato / Rekordbox / Traktor history files (`library-adapters` later).
- Sampler pad triggers, one-shots, or sampler-bank playback.
- Master fader gating (effective level uses deck fader × crossfader only; §4.2).
- Dedicated “now playing” sidecar file (OBS reads the live XSPF; §11).
- Streaming-provider tracks without a stable `tracks.id` (until stream sources exist).
- SoundExchange / PPL submission templates (CSV export is sufficient).
- CUE-sheet export tied to mix recordings.

---

## 4 — Logging Rules

### 4.1 What gets logged

| Source | Logged? | Notes |
|--------|---------|-------|
| Deck play (any deck 0…N−1) | **Yes** | After volume + duration gates (§4.2) |
| Same track played again in one session | **Yes** | Always append; no dedup |
| Sampler pad press / hold / loop | **No** | Even if routed through a deck channel |
| Preview / headphone (PFL) only | **No** | Does not affect idle timer or logging |
| Track loaded but never played | **No** | Must enter qualifying play state |

### 4.2 Qualifying play & entry lifecycle

**Effective output level** (per deck):

```text
effective = deck.volume × crossfader_gain(deck)
```

| Input | Source |
|-------|--------|
| `deck.volume` | Deck channel fader (`DeckUpdated.volume`, 0.0–1.0) |
| `crossfader_gain(deck)` | Equal-power crossfader (`EngineStatus.crossfader`): deck 0 → `cos(xf × π/2)`, deck 1 → `sin(xf × π/2)` |

ponytail: decks 2–3 use gain `1.0` until a 4-deck crossfader model lands (`engine-dsp` currently assigns full gain to lanes ≥ 2).

Gain trim and master volume are **not** included.

**Qualifying play state** (per deck):

```text
playing == true
AND track loaded
AND effective >= history_min_deck_volume
```

Default `history_min_deck_volume` is **0.05** (5% of post-crossfader level).

**Lifecycle:**

1. Deck enters qualifying state → record **`started_at`** (UTC), start duration timer.
2. Continuously qualifying for **`history_min_play_seconds`** (default **5**) → **commit** entry to XSPF (append `<track>` with metadata snapshot).
3. Deck leaves qualifying state (pause, stop, unload, or `effective` below threshold) → set **`ended_at`** (UTC) on the committed entry; rewrite XSPF entry extension.
4. If qualifying state ends **before** `history_min_play_seconds` elapses → **discard**; no entry written.

**Per-entry fields:**

| Field | When set |
|-------|----------|
| `started_at` | Qualifying state begins |
| `ended_at` | Qualifying state ends (or session close while still qualifying) |
| `played_duration_ms` | `ended_at − started_at` (computed) |
| `deck_id` | 0-based deck index |
| `track_id` | Library id when available |
| `location` | File URI for XSPF `<location>` |
| `title`, `creator`, `album`, `duration` | Snapshot at commit time |
| `bpm`, `key`, `isrc` | Snapshot at commit time |

Multiple decks may have overlapping entries (simultaneous play on 2+ decks).

### 4.3 Open entries at session close

When a session closes (idle timeout, manual new session, or app exit):

- Any committed entry without `ended_at` gets `ended_at = close time`.
- Pending entries that never reached `history_min_play_seconds` are discarded.

### 4.4 “Active output” (idle timer)

For session idle detection, a deck is **actively outputting** when it is in **qualifying play state** (§4.2).

**Not** considered:

- Sampler activity.
- Preview / PFL bus.
- Master volume or headphone cue toggles (crossfader **is** included via §4.2).

When **no deck** is actively outputting, the idle clock runs. If idle exceeds `history_session_idle_minutes`, the session **closes**. The next qualifying play starts a **new session** (unless restored — §5.3).

### 4.5 Engine hook

History listens to the **engine event bus** (`DeckUpdated`, `EngineStatus` for crossfader, and related), not Flutter widgets. Implementation lives in `engine-core` or a small `history` module invoked from the cmd/evt worker.

Sampler commands (`SamplerPadPress`, `AssignSamplerTrack`, …) are ignored.

---

## 5 — Session Model

### 5.1 Lifecycle

```text
                    ┌─────────────────────────────────────┐
                    │  Session A (2026-…Z.xspf)           │
  qualifying play ─►│  append / update track entries      │
                    └──────────────┬──────────────────────┘
                                   │
         no active deck output     │
         idle ≥ timeout             │     manual "New session"
         OR manual close            │     OR app exit
                                   ▼
                    ┌─────────────────────────────────────┐
                    │  Session A closed                   │
                    │  next qualifying play → Session B   │
                    └─────────────────────────────────────┘
```

1. **Start** — first committed entry when no active session exists.
2. **Active** — XSPF receives appends and `ended_at` updates.
3. **Idle** — no deck in qualifying play → idle clock runs.
4. **Close** — idle timeout, manual new session, or app shutdown policy (§5.3).
5. **Continue** — qualifying play before idle timeout → same session (idle clock reset).

### 5.2 Session identity

```text
{appSupport}/history/2026-08-27T143022Z.xspf
```

- Filename: UTC session start `YYYY-MM-DDTHHMMSSZ.xspf` (sortable; immutable).
- `<playlist><title>`: default local label, e.g. `2026-08-27 14:30`.
- User rename: updates `<title>` + `history_sessions.title`; filename unchanged.

### 5.3 App restart

On launch, if an **unclosed** session exists and `now − last_activity_at < history_session_idle_minutes`:

- Show dialog: **Restore session** vs **Start new session**.
- **Restore** — resume appending to the same XSPF; idle clock reset.
- **Start new** — mark previous session `closed`, next qualifying play creates a new file.

If `now − last_activity_at ≥ history_session_idle_minutes`:

- Auto-close the previous session (set `ended_at` on open entries, `closed=true`).
- No prompt; next play starts a new session.

`last_activity_at` is persisted in session XSPF extension metadata and `history_sessions`.

### 5.4 Manual session control

| Action | Behavior |
|--------|----------|
| **New session** | Close current session immediately; next qualifying play creates a new XSPF. Requires at least one entry in the current session (Mixxx parity). |
| **Resume session** | Reopen the most recent **closed** session as active (`closed=false`); continue appending to its XSPF. **Only available while no successor session has been created** — i.e. after a session closes (idle, manual new session, or “Start new” on restart) and **before** the next session file exists on disk. Hidden/disabled once a new XSPF is created or any entry is committed to a successor. Does **not** merge two non-empty sessions. |

Typical flow: idle closes Session A → user selects **Resume session** before playing again → Session A reopens → same gig log continues. If the user plays first, Session B is created and Resume is no longer offered.

---

## 6 — Storage (XSPF)

### 6.1 Canonical format

**MIME type:** `application/xspf+xml`  
**Encoding:** UTF-8  
**Namespace:** `http://xspf.org/ns/0/`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
  <title>2026-08-27 14:30</title>
  <creator>Mixar</creator>
  <date>2026-08-27</date>
  <info>Mixar performance session</info>
  <extension application="https://mixar.app/ns/history/1">
    <session id="uuid"
             started_at="2026-08-27T14:30:22Z"
             last_activity_at="2026-08-27T16:05:01Z"
             closed="false"/>
  </extension>
  <trackList>
    <track>
      <location>file:///home/me/Music/track.flac</location>
      <title>Track Title</title>
      <creator>Artist</creator>
      <album>Album</album>
      <duration>360</duration>
      <extension application="https://mixar.app/ns/history/1">
        <entry id="uuid"
               deck="1"
               track_id="…"
               started_at="2026-08-27T14:31:05Z"
               ended_at="2026-08-27T14:36:12Z"
               played_duration_ms="307000"
               bpm="128.0"
               key="Am"
               isrc="USXXX1234567"/>
      </extension>
    </track>
  </trackList>
</playlist>
```

**Mixar extension namespace** (`https://mixar.app/ns/history/1`) holds deck, timestamps, library id, BPM/key/ISRC snapshots. Standard XSPF fields stay populated for third-party readers.

Committed entries may be **updated in place** when `ended_at` is set (same `<entry id="…">`).

### 6.2 Session index (`library.db`)

XSPF is the source of truth for entries. Index for fast UI listing:

```sql
CREATE TABLE history_sessions (
  id               TEXT PRIMARY KEY,
  xspf_path        TEXT NOT NULL UNIQUE,
  title            TEXT NOT NULL,
  started_at       TEXT NOT NULL,
  last_activity_at TEXT NOT NULL,
  closed           INTEGER NOT NULL DEFAULT 0,
  entry_count      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_history_sessions_started ON history_sessions(started_at DESC);
```

**`CollectionType::History`:** not used for auto-recorded sessions. Reserved for possible future **manual import** of external history files into the library. Auto sessions use `history_sessions` + dedicated History UI only.

### 6.3 Write durability

1. Mutate in-memory model.
2. Write `{path}.tmp`.
3. Atomic rename → `{path}.xspf`.
4. Commit `history_sessions` update.

ponytail: full-file rewrite per commit/update; acceptable for typical set sizes (<500 entries).

### 6.4 Storage location

```text
{getApplicationSupportDirectory()}/history/*.xspf
```

Same app-support root as `library.db` and `settings.json` (app id `top.mixar.app`).

---

## 7 — Track Metadata (ISRC)

```sql
-- via sea-orm entity sync
ALTER TABLE tracks ADD COLUMN isrc TEXT;
CREATE INDEX idx_tracks_isrc ON tracks(isrc);
```

| Concern | Rule |
|---------|------|
| **Import / scan** | Read ISRC from file tags via `lofty` when present |
| **History entry** | Snapshot `tracks.isrc` at commit time into XSPF extension |
| **Library UI** | **Track detail only** — not a library table column |
| **Manual edit** | Editable in track detail panel |

Later (non-blocking): `label`, `catalog_number` for PPL-style exports.

---

## 8 — Data Flow & Ownership

```text
 Engine ──► DeckUpdated evt ──► History recorder
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          ▼                           ▼                           ▼
   history/*.xspf            history_sessions              HistorySessionUpdated evt
          │                           │
          └──────────► LibraryTransport / Flutter UI ◄──────┘
```

| Layer | Responsibility |
|-------|----------------|
| `engine-core` (or `history` crate) | Qualifying-play detection, gates, idle timeout, XSPF R/W |
| `library` | Session index, merge/join, save-as-playlist |
| `host-flutter` | FRB: list/get/export/rename/manual session |
| `gui-flutter` | History browser, restore prompt, settings |

Use **`LibraryTransport`** (or `HistoryTransport` if the API grows). No raw FRB from widgets.

---

## 9 — Library & UI

### 9.1 History view

- Sidebar **History**, ordered by `started_at` desc.
- Row: title, date, entry count, span (`first started_at` → `last ended_at`).
- Detail: `#`, `started_at`, `ended_at`, duration, deck, title, artist, BPM, key, ISRC (read-only snapshot).

### 9.2 Actions

| Action | Behavior |
|--------|----------|
| **Rename session** | XSPF `<title>` + index |
| **New session** / **Resume session** | §5.4 |
| **Export…** | §12 |
| **Save as playlist** | Sortable `Playlist` via `collection_tracks` |
| **Delete session** | Remove XSPF + index row (confirm) |
| **Reveal in folder** | Opens `history/` in file manager |

---

## 10 — Settings

Add to `settings.json` / `AppSettings`:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `history_enabled` | `bool` | `true` | Master toggle |
| `history_session_idle_minutes` | `u32` | `5` | Idle timeout before auto-close / new session |
| `history_min_play_seconds` | `u32` | `5` | Minimum qualifying duration before entry is committed |
| `history_min_deck_volume` | `f32` | `0.05` | Minimum **effective** output (`deck.volume × crossfader_gain`) for qualifying play |
| `dim_played_tracks` | `bool` | `true` | Dim library/drive rows present in the open history session |

UI: Settings → Session panel.

When `history_enabled == false`, no entries or session files are written; idle/manual session UI is hidden or disabled; played-track dimming is off regardless of `dim_played_tracks`.

---

## 11 — Live Output (OBS)

No separate now-playing file. External tools (OBS text source, scripts) watch the **active session XSPF** — the most recently modified `.xspf` in `{appSupport}/history/`, or the file referenced by the open `history_sessions` row where `closed = 0`.

Recommended integration pattern:

- Poll the active XSPF on an interval.
- Parse the last `<track>` with a committed entry; display `creator - title`.
- Optionally show elapsed time from `started_at`.

Document the path in user-facing help; no Mixar-specific OBS plugin required for MVP.

---

## 12 — Export (Derived Formats)

Export writes a **copy**; canonical XSPF is unchanged.

| Format | Use case |
|--------|----------|
| **XSPF** | On disk; reveal / copy |
| **CSV** | Spreadsheets, venue / licensing workflows |
| **M3U8** | Other players / tools |
| **Plain TXT** | Mixcloud / SoundCloud setlists |

CSV columns:

```text
position, started_at, ended_at, played_duration_ms, deck, title, artist, album, bpm, key, isrc, file_path, track_id
```

---

## 13 — Phased Delivery

| Phase | Deliverable |
|-------|-------------|
| **H1** | Recorder, volume/duration gates, idle split, XSPF R/W with `started_at`/`ended_at`, `history_sessions`, ISRC on `tracks` |
| **H2** | History UI, settings, restore prompt, export CSV/M3U8/TXT |
| **H3** | Manual new/resume session, save-as-playlist, rename/delete, 4-deck verification |
| **H4** | Import adapters (Rekordbox TXT, generic M3U8) |

---

## 14 — Acceptance Criteria

**History MVP complete when:**

1. A deck play with effective output ≥ threshold for ≥ `history_min_play_seconds` commits an entry with correct `started_at`, deck, and metadata.
2. When play ends, the entry receives `ended_at` and correct `played_duration_ms`.
3. Plays below duration or effective-output threshold produce **no** entry.
4. Re-playing the same track appends a **second** entry.
5. Sampler triggers create **no** entries; PFL does not affect idle or logging.
6. After idle timeout (default 5 min), the next qualifying play opens a **new** session.
7. Manual **New session** and **Resume session** work as specified (Resume only before a successor session exists).
8. Crossfader fully cut (effective below threshold) prevents logging even if deck is playing.
9. On restart inside idle window, restore prompt appears; declining starts fresh.
10. All four settings persist and affect behavior.
11. ISRC from tags appears in track detail and history entry snapshots.
12. CSV and M3U8 export open in external tools.
13. OBS can read the active session from `{appSupport}/history/*.xspf`.

---

## 15 — References

### Competitors & formats

| Resource | URL |
|----------|-----|
| Serato History | https://support.serato.com/hc/en-us/articles/223455687-History |
| Mixxx Library §4.12 History | https://manual.mixxx.org/2.3/en/chapters/library.html |
| XSPF specification | https://www.xspf.org/ |
| Engine DJ User Guide | https://cdn.inmusicbrands.com/engine/Engine_DJ_User_Guide_v2.2.0.pdf |
| VirtualDJ history files | https://virtualdj.com/forums/238724/VirtualDJ_Technical_Support/How_do_VDJs_history_files_work__(tracklist_txt____m3u_files).html |

### This repository

| Resource | Path |
|----------|------|
| Collections model | `docs/tech-spec.md` §10 |
| Engine events | `crates/engine-api/src/payload.rs` |
| Track entity | `crates/library/src/entity/tracks.rs` |
| App settings | `crates/host-flutter/src/api/settings.rs` |
| Phase 5 roadmap | `docs/deck-spec.md` §10 |
