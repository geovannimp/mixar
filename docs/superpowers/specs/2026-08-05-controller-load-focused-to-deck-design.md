# Controller Load Focused Track to Deck

**Date:** 2026-08-05  
**Status:** accepted  
**Context:** DDJ-400 library browse + LOAD buttons; FE already owns table focus via `LibraryNavigation::navigate*`.

## Goal

Hardware LOAD deck 1/2 loads the currently focused library table row (collection track **or** filesystem path), same as the UI load buttons.

## Decision

| Decision | Choice |
|----------|--------|
| Architecture | Approach A: FE resolves focus |
| Wire | `Kind::LoadFocusedToDeck` + `EvtBody::LoadFocusedToDeck { deck: u16 }` (0-based) |
| Controller leaves | `LibraryNavigation::load_to_deck_1` / `load_to_deck_2` (master section) |
| DDJ MIDI | Mixxx LoadSelectedTrack: ch7 note `0x46` / `0x47` |
| Empty focus | No-op |
| Worker | Ignore `LoadFocusedToDeck` on cmd bus (UI-only evt kind) |

## Flow

1. LOAD press → controller `LibraryEvt` on library evt bus.
2. FE `library-store` reads `focusedLoad` (synced from panel table + focus index).
3. Prefer `trackId` → `loadLibraryTrackToDeck`; else `path` → `loadPathToDeck`.

## Out of scope

Shift+LOAD, decks 3+, host-side focus mirror.
