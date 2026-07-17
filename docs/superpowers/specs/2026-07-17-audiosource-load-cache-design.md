# AudioSource load API + engine decode cache

**Issue:** [#78](https://github.com/geovannimp/rust-dj-engine/issues/78)  
**Date:** 2026-07-17  
**Related:** #72 (mixer lanes), #67 (volume normalizer)

## Problem

`Engine::load_track` takes `Arc<LoadedAudio>`, so the engine never sees track identity or tags. ReplayGain / loudness policy is applied in the GUI, and decode caching also lives in Tauri.

## Goals

1. Rename enum `LibrarySource` → **`AudioSource`**.
2. Rename trait `audio_core::AudioSource` → **`LoadableAudio`** (`load() -> LoadedAudio`).
3. **`Engine::load_track(deck_id, source: AudioSource)`** only — engine decodes via `LoadableAudio`.
4. **Engine-owned PCM decode cache** keyed by `TrackId`.
5. **`TrackMetadata.replaygain_track_gain_db: Option<f64>`** from lofty `ReplayGainTrackGain`.
6. Optional **`TrackMetadata.loudness_lufs`** for analyzed loudness stamped onto the source before load (prefer ReplayGain when deriving channel auto-gain).
7. Mixer channel derives auto-gain from loudness + `set_normalizer_target`; drop GUI computation of `auto_gain_db` at load.
8. Remove Tauri’s `LoadedAudio` track cache (waveform caches may still decode by path).

## Non-goals

- Streaming playback.
- Writing ReplayGain tags.
- Moving waveform overview/detail cache into the engine.

## Naming

| Old | New |
|-----|-----|
| `library_core::LibrarySource` | `library_core::AudioSource` |
| `audio_core::AudioSource` (trait) | `audio_core::LoadableAudio` |
| `LoadedAudio` | unchanged (PCM buffer only; no loudness field) |

Variants stay `AudioSource::File(FileAudioSource)` / `AudioSource::Stream(StreamAudioSource)`.

## Data flow

```text
Library / tags
  → TrackMetadata { …, replaygain_track_gain_db, loudness_lufs? }
  → AudioSource::File { id, metadata, path }

App
  → engine.set_normalizer_target(enabled ? Some(target) : None)
  → engine.load_track(deck_id, source)

Engine
  → cache lookup by TrackId
  → miss: source.load() → Arc<LoadedAudio> → store
  → deck.load(pcm)
  → channel.set_loudness_lufs(loudness_from_metadata(source.metadata()))
```

**Loudness preference:**  
`replaygain_track_gain_db` → convert with `loudness_lufs_from_replaygain_track_gain_db`  
else `metadata.loudness_lufs`  
else `None` (auto gain 0).

## Components

### audio-core

- Rename trait to `LoadableAudio`.
- `LoadedAudio` remains samples + rate + channels + `source_id` only.

### library-core

- Rename `LibrarySource` → `AudioSource`.
- Add `replaygain_track_gain_db` and `loudness_lufs` to `TrackMetadata`.
- Implement `LoadableAudio` for `FileAudioSource` / `StreamAudioSource` / `AudioSource`.

### library

- `read_tags` fills `replaygain_track_gain_db` (reuse existing parse helpers).
- Persist `replaygain_track_gain_db` on `tracks` (SeaORM entity + sync).
- When analyzing, keep `track_analysis.loudness_lufs`; callers may copy onto metadata at load.

### engine-dsp

- `MixerChannel`: `loudness_lufs`, `target_lufs: Option`, cached `auto_gain_db`; recompute on setters.
- `Mixer::set_normalizer_target`.

### engine-core

- Decode cache: `HashMap<TrackId, Arc<LoadedAudio>>` (Weak or prune on stop).
- `load_track(deck_id, AudioSource)`.
- `set_normalizer_target(Option<f32>)`.
- `deck_auto_gain_db(deck_id)` for UI sync.

### gui-app

- Pass `AudioSource` into `load_track`.
- Stamp analysis loudness onto metadata when known.
- Drop track `LoadedAudio` cache; keep overview/detail caches (decode via path or `LoadableAudio` as needed).
- Apply normalizer target on engine start / settings restart.

## Acceptance

1. Engine load requires `AudioSource` and sees metadata.
2. ReplayGain tag → metadata → channel auto-gain when normalizer on.
3. Same `TrackId` reload hits engine cache.
4. `AudioSource::File(FileAudioSource::from_path(...))` works for examples/tests.
5. Focused tests cover rename, metadata→gain, and cache hit.
