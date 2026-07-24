# Sampler Banks Design

**Date:** 2026-07-22  
**Issue:** [#47](https://github.com/geovannimp/rust-dj-engine/issues/47) — Sampler pad mode (S3)  
**Spec refs:** `docs/deck-spec.md` §5.5 / §5.10 S3  
**Status:** Approved product decisions (pending implementation plan)

## Goal

Replace session-scoped sampler slots with **named, persisted sample banks** (8 slots each). Decks select a bank (default from settings; last-used remembered on the track after a sample is actually played). Each bank has an optional **play mode** (`oneshot` \| `hold` \| `loop`); **`NULL` inherits** the app-wide sampler play mode from settings. Sample playback is **loudness-normalized** via the shared volume-normalizer config.

## Competitor context (why this shape)

| Product | Capacity | Scope | Persistence |
|---------|----------|-------|-------------|
| Serato | 4×8 banks | Global bank | Sampler save / crates |
| Virtual DJ | 8 pads + banks | Global | Sampler bank |
| Mixxx | Up to 64 sampler decks | Global | `samplers.xml` + bank XML files |
| Traktor | Sample / remix decks | Global | User-managed |

This design follows the **Serato / VDJ bank × 8 pads** model, with Mixxx-like persistence of assignments, plus **per-deck default bank** and **per-track last-used bank** (only after a real trigger).

## Requirements

| Decision | Choice |
|----------|--------|
| Persistence | Yes — `library.db`, not session-only |
| Bank size | Fixed **8 samples** per bank |
| Capacity beyond 8 | Multiple **named banks**, not a larger pad grid |
| Bank naming | Required display name (user-editable) |
| Play mode (settings) | App-wide **default sampler play mode**: `oneshot` \| `hold` \| `loop` |
| Play mode (bank) | Per bank: `NULL` \| `oneshot` \| `hold` \| `loop` (not per slot). **`NULL` = inherit settings** (do not store a `"default"` sentinel) |
| Deck default bank | Configurable in **app settings** (per deck A/B) |
| Track association | Store **last used bank** on the track, **only if a sample was actually played** while that bank was active on a deck |
| Pad UI | Same 8-pad grid in Sampler pad mode |
| Audio storage | References only (`track_id` and/or path); no embedded PCM in DB |
| Loudness | All samples **normalized** using the existing volume-normalizer settings (`volume_normalizer_enabled`, `target_lufs`) — same rules as deck auto-gain |

### Play modes

**Effective play mode** for a bank:

```text
if bank.play_mode IS NULL:
    use settings.sampler_play_mode
else:
    use bank.play_mode
```

| Bank `play_mode` | Meaning |
|------------------|---------|
| `NULL` | Inherit settings `sampler_play_mode` (until the user sets an explicit mode on this bank) |
| `oneshot` | Override: pad down starts playback; sample plays to end (or until retrigger policy). Pad up does not stop. |
| `hold` | Override: pad down starts; pad up / key-up stops (gate). |
| `loop` | Override: pad down starts looping the sample; pad up stops (see open detail). |

| Settings `sampler_play_mode` | Behavior when bank `play_mode` is `NULL` |
|------------------------------|------------------------------------------|
| `oneshot` | Same as bank `oneshot` |
| `hold` | Same as bank `hold` |
| `loop` | Same as bank `loop` |

- New banks: `play_mode = NULL` (inherit).
- Settings factory default: `sampler_play_mode = oneshot`.
- Changing settings updates effective behavior for **all banks with `play_mode IS NULL`**; banks with an explicit mode are unchanged.
- Resetting a bank to “use default”: write `NULL`, never the string `"default"`.

### Loudness normalization

Samples use the same normalizer config as decks ([volume normalizer design](./2026-07-16-volume-normalizer-design.md)):

| Condition | Gain applied to sample voice |
|-----------|------------------------------|
| Normalizer **on** + sample has `loudness_lufs` (library analysis / ReplayGain) | `auto_gain_db = clamp(target_lufs - loudness_lufs, ±12 dB)` |
| Normalizer **off**, or no loudness available | `auto_gain_db = 0` |

- Apply at assign/load into the sampler (and re-apply when settings enable/target change for currently loaded bank slots), not by baking into files.
- Path-only / unanalyzed samples: no loudness → unity gain until analyzed.
- No separate sampler target LUFS for MVP — one shared `target_lufs` / enable flag in settings.

### Last-used bank on track

1. Deck is in Sampler mode with bank **B** selected.
2. User **triggers** a pad that has an assigned sample (successful play start).
3. Persist `tracks.last_sampler_bank_id = B` for the **loaded track** on that deck.
4. Merely selecting a bank, assigning slots, or switching pad mode **does not** write last-used.
5. On next load of that track to a deck: prefer `last_sampler_bank_id` if the bank still exists; else fall back to that deck’s **settings default bank**.

### Sampler settings

| Field | Role |
|-------|------|
| `sampler_play_mode` | App-wide default: `oneshot` \| `hold` \| `loop`. Used by every bank whose `play_mode` is `NULL`. |
| `deck_default_sampler_bank_id` | Per deck A/B — which bank is selected when no track last-used applies. |

- Deck default bank used when: no track loaded, or track has no last-used bank, or last-used bank was deleted.
- Changing deck-default bank does not rewrite existing track last-used values.
- Changing `sampler_play_mode` does not rewrite bank rows; banks with `NULL` play_mode resolve at runtime.

## Data model

```sql
CREATE TABLE IF NOT EXISTS sampler_bank (
    id              TEXT NOT NULL PRIMARY KEY,
    name            TEXT NOT NULL,
    play_mode       TEXT,              -- NULL = inherit settings; else 'oneshot' | 'hold' | 'loop'
    sort_index      INTEGER NOT NULL DEFAULT 0,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sampler_slot (
    bank_id         TEXT NOT NULL REFERENCES sampler_bank(id) ON DELETE CASCADE,
    slot_index      INTEGER NOT NULL,  -- 0..BANK_SIZE-1; validated in app code
    track_id        TEXT REFERENCES tracks(id) ON DELETE SET NULL,
    path            TEXT,              -- fallback when not a library track
    label           TEXT,
    updated_at      TEXT NOT NULL,
    PRIMARY KEY (bank_id, slot_index)
    -- slot_index range (0..BANK_SIZE-1) enforced in app code, not SQL CHECK,
    -- so bank size can change without a schema migration
);

-- Last bank used after a sample was played while this track was loaded
-- (column on tracks; SeaORM entity + schema sync — no separate prefs table)
ALTER TABLE tracks ADD COLUMN last_sampler_bank_id TEXT
    REFERENCES sampler_bank(id) ON DELETE SET NULL;
```

**Slot assignment rule:** prefer `track_id` when assigned from library; keep `path` for filesystem-only drops. Resolve audio via library when `track_id` is set.

**Bank size:** `BANK_SIZE = 8` as a single app constant (library + engine + GUI). Reject out-of-range `slot_index` in assign/clear/trigger APIs; do not encode the bound in SQL.

**Empty bank:** allowed (all slots empty). Trigger on empty slot is a no-op (toast optional).

## Runtime / engine

```text
App settings
  ├── sampler_play_mode          (oneshot | hold | loop)
  ├── volume_normalizer_enabled / target_lufs   (shared with decks)
  └── default bank per deck

Deck (Sampler pad mode)
  └── active_bank_id
        ├── from track.last_sampler_bank_id (if set + bank exists)
        └── else settings default for that deck

Sampler DSP
  └── 8 voices for the active bank’s slots
        ├── effective_play_mode =
        │     settings.sampler_play_mode  if bank.play_mode IS NULL
        │     else bank.play_mode
        └── per-slot auto_gain_db from normalizer config + sample loudness
```

- Switching bank on a deck reloads the 8 slot assignments into the engine (decode/cache as today).
- Both decks may share the same bank; triggers are global one-shots into the shared sampler bus (current architecture). Per-deck mute of sampler output is out of scope.
- Pad / keyboard 1–8 respect **effective** play mode (`hold` / `loop` need pointer/key up → stop).
- Changing normalizer enable/target recomputes `auto_gain_db` for loaded sampler slots (same as loaded decks).

## API surface (Tauri)

```text
# Banks
list_sampler_banks() -> SamplerBankSummary[]
create_sampler_bank(name, play_mode?) -> SamplerBank  # play_mode omitted/null = inherit
update_sampler_bank(bank_id, name, play_mode)  # play_mode null = inherit
delete_sampler_bank(bank_id)

# Slots
assign_sampler_slot(bank_id, slot, track_id? | path?)
clear_sampler_slot(bank_id, slot)
get_sampler_bank(bank_id) -> { bank, slots[8] }

# Deck binding
set_deck_sampler_bank(deck_id, bank_id)   # runtime selection
trigger_sampler_pad(deck_id, slot)        # on success → maybe persist last_used
end_sampler_pad(deck_id, slot)            # hold / loop release

# Settings (existing save_settings)
sampler_play_mode: oneshot | hold | loop          # app-wide default for banks with NULL play_mode
deck_default_sampler_bank_id: [Option<bank_id>; 2]
```

`EngineStatus` (or deck status) exposes: `active_sampler_bank_id`, bank name, bank `play_mode`, **effective** play mode, and the 8 slot infos for the active bank.

## GUI

- Sampler pad mode tabs remain 8 pads.
- Bank selector (dropdown or ◀/▶) above/beside pads when mode = Sampler: shows bank **name**, cycles banks.
- Empty pad: slot number + drop target; filled: label + duration affordance.
- Library: “Assign to sampler pad N” targets the **active bank** of the focused deck (or prompt if none).
- Settings: app-wide **sampler play mode**; default sampler bank per deck.
- Bank play-mode control includes **Default** (stores `NULL`) plus Oneshot / Hold / Loop.

## Non-goals (this design)

- Per-slot play mode (bank-level only).
- Per-track sample packs that replace the global bank model.
- Embedding audio blobs in SQLite.
- MIDI mapping (#49) — consumes the same trigger commands later.
- Sampler volume / choke groups / sync-to-BPM (future).
- Separate sampler-only LUFS target (MVP shares deck normalizer settings).

## Success criteria

1. Sample banks and slots survive app restart via `library.db`.
2. Each bank has a name and optional play mode (`NULL` \| `oneshot` \| `hold` \| `loop`); `NULL` follows settings `sampler_play_mode`.
3. Settings expose app-wide sampler play mode and a configurable default bank per deck.
4. Changing settings play mode updates all banks with `NULL` play_mode without rewriting bank rows.
5. Loading a track restores last-used bank only when a sample was previously played for that track; otherwise uses deck default.
6. Selecting a bank without triggering a sample does not update track last-used.
7. Pad grid stays 8 slots; more samples = more banks.
8. Triggered samples respect volume-normalizer settings (shared `target_lufs` / enable); missing loudness → no auto gain.

## Open details (resolve in implementation plan)

| Topic | Tentative default |
|-------|-------------------|
| `loop` pad interaction | Hold-to-loop (down = start loop, up = stop), matching `hold` ergonomics |
| Retrigger while playing (`oneshot`) | Restart voice from start (polyphonic steal if needed) |
| Delete bank that is a deck default | Fall back to first remaining bank by `sort_index`, or `None` |
| Delete bank that is a track’s last-used | `ON DELETE SET NULL` → next load uses deck default |
| Max banks | Soft limit none for MVP; UI can list all |
| Seed data | No DB seed. If no banks exist, start an unsaved draft (same as UI create); persist on first sample assign / bank edit |

## Migration from current MVP

Current session-only `AppState.sampler_slots` / engine sampler:

1. Add tables + library APIs.
2. On engine start, load deck defaults if set; otherwise start an unsaved draft when no banks exist.
3. Point assign/trigger/clear commands at `bank_id` + slot.
4. Wire settings + track last-used write on successful trigger.
5. Remove session-only slot array as source of truth (keep as runtime cache of active bank).

## Files likely touched

- `crates/library/` — entities, schema sync, bank/slot CRUD, `tracks.last_sampler_bank_id`
- `crates/engine-dsp/src/sampler.rs` — play modes (hold/loop stop); per-slot auto-gain from normalizer
- `crates/engine-core/src/engine.rs` — bank load / trigger / end
- `apps/gui-app/src-tauri/src/deck_sampler.rs` — commands + last-used write
- `apps/gui-app/src-tauri/src/lib.rs` — settings fields
- `apps/gui-app/src/components/DeckPadsPanel.tsx` — bank selector + hold/loop
- `apps/gui-app/src/stores/engineStore.ts` / `types.ts`
- `docs/deck-spec.md` — update S3 persistence note from “future” to this model
