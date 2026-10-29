// The bus bootstrap is what turns one Rust event stream into UI state. Feed it
// a typed event and assert the provider graph it drives, without a live engine.
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/providers/shell/bus_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

import 'support/bus_events.dart';

/// The bootstrap needs a [Ref]; this hands one out for tests.
final _capturedRef = Provider<Ref>((ref) => ref);

void main() {
  test('a deck update on the bus reaches the engine mirror', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    expect(container.read(engineUiProvider).channelFor(0).volume, 1.0);

    dispatchBusEvent(
      container.read(_capturedRef),
      deckUpdatedEvent(
        deck: 0,
        update: deckUpdated(deck: 0, volume: 0.25, playing: true),
      ),
    );

    final state = container.read(engineUiProvider);
    expect(state.channelFor(0).volume, 0.25);
    expect(state.isPlaying(0), isTrue);
  });

  test('an error event lands on the notice surface', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    dispatchBusEvent(
      container.read(_capturedRef),
      Event.error(
        const Error(message: 'No track loaded', trackId: null, deck: 0),
      ),
    );

    final notice = container.read(appNoticeProvider);
    expect(notice?.kind, AppNoticeKind.error);
    expect(notice?.message, 'No track loaded');
  });
}
