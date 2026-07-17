# Mixer Channel Graph Design

**Issue:** [#72](https://github.com/geovannimp/rust-dj-engine/issues/72)  
**Spec refs:** `docs/deck-spec.md` §5.8; VU level-meter design; volume-normalizer design  
**Date:** 2026-07-16

## Goal

Make `Deck` a playback/transport unit and move each per-deck mixer strip into a mixer-owned `dasp_graph` channel node.

## Architecture

Each `MixerLane` is a graph node that owns its deck and strip:

```text
MixerLane (deck → dry stash → strip) → Sum → master bus
                    │
                    └→ pre-fader VU / headphone PFL
```

`MixerLane` applies this fixed order inside each graph chunk (after a full-callback dry deck render):

```text
auto gain + trim → three-band EQ → filter → VU/PFL snapshot → channel fader → crossfader → sum
```

The crossfader remains after the channel fader and does not affect the PFL tap. A single node owns the complete channel strip; future FX may be added inside the node or motivate splitting it into insert/fader nodes later.

## Boundaries

### `Deck`

Owns track audio, transport, tempo, looping, cue-point behavior, resampling, and a dry output buffer. Loading takes only decoded audio. It does not own or apply gain, EQ, filter, channel volume, channel metering, headphone cue routing, or crossfader state.

### `MixerChannel`

Strip DSP owned by `MixerLane`:

- `gain_trim_db`
- `auto_gain_db`
- three-band EQ
- DJ filter
- channel volume
- headphone cue enablement
- latest pre-fader peaks
- latest pre-fader stereo buffer
- current crossfader gain supplied by `Mixer`

### `MixerLane`

Graph node that owns `Deck` + `MixerChannel`. On `begin_render`, renders a full dry deck buffer; each `Node::process` chunk runs that audio through the strip. Access: `mixer.lane(i).deck()` / `.channel()`.

### `Mixer`

Owns lane nodes in the graph (`Lane → Sum`), sets crossfader gains, runs the graph, sums PFL from enabled lanes, and routes master/cue buses.

### `DspEngine` and `Engine`

Playback operations continue through `Deck`. Mixer controls and level snapshots route through `Mixer` channels. `Engine::load_track` loads dry audio into the deck and sets that channel's `auto_gain_db`; existing Tauri command names and UI status fields remain stable.

## State and Errors

- `Mixer::new(sample_rate, buffer_size, num_lanes, resampler_quality)` creates one lane + source/channel pair per deck.
- Invalid channel indexes return the same invalid-deck errors at the engine API boundary.
- Loading a track updates channel auto gain but preserves manual channel controls.
- Unloading clears playback only; auto gain is reset to `0 dB` to prevent stale normalization on a later empty/reloaded deck.
- Gain and EQ/filter ranges retain their existing validation/clamping behavior.
- No duplicate gain stage remains in `Deck`.

## Testing

- `Deck` tests prove output is dry and loading no longer accepts auto gain.
- `MixerChannel` tests cover defaults, gain composition, EQ/filter ownership, pre-fader metering/PFL, fader behavior, and validation.
- `Mixer` tests prove the graph topology, channel processing, cue pre-fader behavior, and crossfader routing.
- `engine-core` tests cover load-time auto gain and mixer-control delegation.
- Existing workspace tests and formatting/type checks must pass.

## Non-goals

- Changing volume-normalizer math or settings behavior.
- Persisting gain trim per track.
- Adding FX slots or crossfader assignment.
- Changing public Tauri command names or GUI data models.
