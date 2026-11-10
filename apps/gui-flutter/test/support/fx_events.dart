import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show FxKind, FxSlotState;

import 'bus_events.dart';

/// Echo as the engine publishes it; `divisionNorm` is param 0 (0 = 1/16 … 1 = 2/1).
FxSlotState echoSlot({
  bool enabled = false,
  double wet = 0.5,
  double divisionNorm = 0.5,
}) => FxSlotState(
  kind: FxKind.echo,
  enabled: enabled,
  wet: wet,
  params: Float32List.fromList([divisionNorm, 0.5, 0.0]),
);

/// Republishes a whole FX unit the way a deck update carries it.
///
/// [fxRouted] is spelled out at every call site on purpose: the FX bar tests
/// assert the routed master unit, while the engine-cmd tests only care about
/// the slots, so neither file can drift into the other's fixture by default.
void applyDeckFx(
  ProviderContainer container,
  List<FxSlotState> fx, {
  int deck = 0,
  bool fxRouted = false,
}) {
  container
      .read(engineUiProvider.notifier)
      .apply(
        deckUpdatedEvent(
          deck: deck,
          update: deckUpdated(deck: deck, fx: fx, fxRouted: fxRouted),
        ),
      );
}
