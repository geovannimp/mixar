import 'package:gui_flutter/core/models/bus_enums.dart';

/// Index of the Beat FX unit inside the shared chain; the remaining capacity is
/// reserved for the future preset / chain editor.
const int kBeatFxSlot = 0;

/// Beat divisions offered by the BEAT ◀ ▶ stepper, in engine knob order
/// (`engine_dsp::fx::echo::ECHO_DIVISIONS`).
const kFxDivisionLabels = ['1/16', '1/8', '1/4', '1/2', '3/4', '1/1', '2/1'];

/// Display label for the FX selector.
String fxKindLabel(FxKind kind) => switch (kind) {
  FxKind.off => 'Off',
  FxKind.echo => 'Echo',
  FxKind.reverb => 'Reverb',
};

/// Beat divisions of `kind`; empty for effects with no beat-synced time, which
/// hides the BEAT stepper (the engine ignores tempo for those).
List<String> fxDivisionLabels(FxKind kind) =>
    kind == FxKind.echo ? kFxDivisionLabels : const [];

/// Engine knob value (param 0) of beat division `index`.
double fxDivisionNorm(int index) => index / (kFxDivisionLabels.length - 1);

/// Nearest beat division index for an engine knob value.
int fxDivisionIndex(double norm) =>
    (norm.clamp(0.0, 1.0) * (kFxDivisionLabels.length - 1)).round();

/// Engine default knob value for the beat division (echo TIME `1/2` beat,
/// `engine_dsp::fx::ECHO_DEFAULT_DIVISION`). Used before the first snapshot.
const kFxDefaultDivisionNorm = 0.5;

/// Beat division index carried by a slot's published knob values (param 0).
///
/// Falls back to the engine default division so the readout never advertises
/// `1/16` while the engine is sitting at `1/2` (no snapshot yet).
int fxDivisionIndexFrom(List<double>? params) => fxDivisionIndex(
  params == null || params.isEmpty ? kFxDefaultDivisionNorm : params.first,
);
