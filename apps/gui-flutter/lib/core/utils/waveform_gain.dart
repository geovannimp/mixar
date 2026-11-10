import 'package:gui_flutter/core/models/engine_ui.dart';

/// Per-band amplitude multipliers, 1.0 = flat.
typedef WaveformBandGain = (double low, double mid, double high);

const flatWaveformBandGain = (1.0, 1.0, 1.0);

/// The deck's EQ knobs as waveform amplitude multipliers.
///
/// Engine knobs are 0..1 with 0.5 flat, so a flat channel maps to 1.0 and the
/// lane shows the recorded amplitudes; killing a band (0) collapses its bars
/// and a boosted band (1.0) doubles them. Mirrors Mixxx, which shapes its
/// waveforms from the same EQ the deck hears.
WaveformBandGain waveformBandGainFor(MixerChannelUi channel) => (
  (channel.eqLow * 2).clamp(0.0, 2.0),
  (channel.eqMid * 2).clamp(0.0, 2.0),
  (channel.eqHigh * 2).clamp(0.0, 2.0),
);

/// The lane painter's amplitude multipliers for [channel].
///
/// Shaped follows the deck's **gain** knob and EQ, never the channel fader:
/// gain is how hot the deck arrives, so the lane tracks it the way it tracks
/// the EQ, while the fader stays a mix move that leaves the shape alone. Unity
/// is the knob centre (0.5), doubling at full.
///
/// With [shaped] off (Settings → Waveform) the lane draws the stored peaks at
/// unity, so moving the gain or an EQ knob leaves it unchanged too.
({double gain, WaveformBandGain bandGain}) waveformGainsFor(
  MixerChannelUi channel, {
  bool shaped = true,
}) => shaped
    ? (
        gain: (channel.gainTrim * 2).clamp(0.0, 2.0),
        bandGain: waveformBandGainFor(channel),
      )
    : (gain: 1.0, bandGain: flatWaveformBandGain);
