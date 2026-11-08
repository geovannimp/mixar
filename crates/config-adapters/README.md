# config-adapters

Third-party application **settings** adapters for
[Mixar](https://github.com/geovannimp/mixar).

`library-adapters` migrates music libraries; this crate migrates the settings of
other DJ applications into Mixar's native configuration types and writes them
back. Adapters are pure, settings-time code: nothing here is called from the
audio callback or owns audio state. `curve-core` stays free of third-party
parsing; the adapter depends on `curve-core` for the curve model and on
`audio-core` for `CrossfaderCurveBehavior`.

Implemented adapters:

- `virtualdj` (default feature) — the four VirtualDJ crossfader options.

## VirtualDJ crossfader

| option | native |
|--------|--------|
| `crossfaderCurve` | preset name, `custom`, or a `0..=1` amount |
| `crossfaderCustom` | `x=[gainA,gainB]/…` point script |
| `crossfaderHamster` | `CrossfaderCurveBehavior::hamster` |
| `crossfaderDisable` | `CrossfaderCurveBehavior::disabled` |

```rust
use config_adapters::virtualdj::{
    export_crossfader, import_crossfader, VirtualDjCrossfaderConfig,
};

let config = VirtualDjCrossfaderConfig {
    crossfader_curve: "0.3".to_string(),
    ..VirtualDjCrossfaderConfig::default()
};

let native = import_crossfader(&config)?;      // CurveGroup + CurveGroupRef + flags
let options = export_crossfader(&native)?;     // VirtualDJ-compatible options
# Ok::<(), config_adapters::VirtualDjError>(())
```

`VirtualDjCrossfaderConfig::from_options` reads any `settings.xml`-derived
key/value map (unknown keys are ignored). Automatic discovery and backup import
of VirtualDJ's settings file are follow-ups.

### Preset mapping

VirtualDJ's preset geometries are not public, so the adapter pins a documented
mapping. Native ids match `engine-core`'s crossfader preset registry.

| VirtualDJ | native | notes |
|-----------|--------|-------|
| `Smooth` | `constant_power` | equal-power law |
| `Default`, `Linear` | `linear` | `A = 1 - x`, `B = x` |
| `Cut` | `fast_cut(0.15)` | fades over the last/first 15% |
| `Scratch` | `fast_cut(0.05)` | fades over the last/first 5% |
| `Full` | `fast_cut(0.01)` | approximate hard cut |
| `Disabled` | `constant_power` + disabled | curve bypassed |
| amount `v` | `fast_cut(1 - v)` | the VirtualDJ amount law |
| amount `v%` | `fast_cut(1 - v / 100)` | percent form |
| `Custom` | `virtualdj_custom` group | parsed from `crossfaderCustom` |

A numeric `crossfaderCurve` is the fraction of travel during which the outgoing
deck stays at full gain, which is exactly the `fast_cut` fade length
`cut = 1 - v`. `0` is the linear law; `1` is a full cut, imported as the `Full`
approximation because the native preset rejects a zero-length fade.

### Custom point syntax

`crossfaderCustom` lists `position=[gainA,gainB]` points. `/` separators and
whitespace are optional, so the concatenated form is valid too:

```text
0=[1,0] / 0.25=[0.5,0.5] / 1=[0,1]
0=[1,0]0.25=[0.5,0.5]1=[0,1]
```

Import requires a point at `x=0`, one at `x=1`, strictly increasing positions,
and gains in `0..=1`. Malformed tokens, unknown preset names, out-of-range
amounts, and invalid booleans all fail with a `VirtualDjError` that names the
offending value.

### Export lossiness

VirtualDJ applies a preset, or draws straight lines between the custom points.
Export writes a preset name when the selection is one VirtualDJ knows; every
other curve is written as a point script sampled at the **union of the A and B
x-points**, so both decks keep their independent breakpoints. Piecewise-linear
curves (custom points and `fast_cut`) therefore roundtrip unchanged, while
non-linear segments — the equal-power Hermite fit, Bézier handles, and
exponential/logarithmic tapers — are flattened to their sampled values and are
lossy.
