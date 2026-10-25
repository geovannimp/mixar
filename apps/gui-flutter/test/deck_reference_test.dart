//! Deck reference: sounding BPM/key math (tempo fader + Key Shift), the
//! debounced push to the library, and the reload it triggers for an open smart
//! playlist.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/deck_reference.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

const _track = LibraryTrackSummary(
  id: 'track-1',
  displayName: 'Peak',
  path: '/music/peak.wav',
  bpm: 128,
  key: 'Am',
);

const _smart = LibraryCollectionSummary(
  id: 'smart-1',
  name: '±6% + key',
  kind: 'smart_playlist',
);

const _folder = LibraryCollectionSummary(
  id: 'folder:/music',
  name: 'Music',
  kind: 'folder',
  path: '/music',
  trackCount: 2,
);

/// Notifier stub: the snapshot is whatever the test set last.
class _FakeEngineUi extends EngineUi {
  new(this.initial);

  final EngineUiSnapshot initial;

  @override
  EngineUiSnapshot build() => initial;

  void set(EngineUiSnapshot next) => state = next;
}

EngineUiSnapshot _snapshot({
  double speed = 0.55,
  bool keyLock = true,
  double keyShift = 2,
}) => EngineUiSnapshot(
  running: true,
  trackPaths: const {0: '/music/peak.wav'},
  trackIds: const {0: 'track-1'},
  playing: const {0: true},
  speeds: {0: speed},
  tempoRanges: const {0: 0.08},
  keyLocks: {0: keyLock},
  keyShifts: {0: keyShift},
);

class _BridgeProbe extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref
      ..watch(deckReferenceBridgeProvider)
      ..watch(collectionTracksProvider);
    return const SizedBox.shrink();
  }
}

void main() {
  test('sounding BPM and key follow the fader, key shift and key lock', () {
    EngineUiSnapshot snapshot({bool keyLock = true, double speed = 0.55}) =>
        _snapshot(speed: speed, keyLock: keyLock);

    ProviderContainer containerFor(EngineUiSnapshot initial) {
      final container = ProviderContainer(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          engineUiProvider.overrideWith(() => _FakeEngineUi(initial)),
          deckLibraryTrackProvider.overrideWith((ref, deckId) => _track),
        ],
      );
      addTearDown(container.dispose);
      return container;
    }

    // normToSpeedRatio(0.55, 0.08) = 1 + (0.5 - 0.55) × 2 × 0.08 = 0.992,
    // so the deck sounds 128 × 0.992 = 126.976 BPM.
    final locked = containerFor(snapshot()).read(deckReferencesProvider);
    expect(locked, hasLength(2));
    expect(locked.first.deckId, 0);
    expect(locked.first.playing, isTrue);
    expect(locked.first.bpm, closeTo(126.976, 1e-9));
    // Key lock holds the pitch, so only the +2 semitone shift applies.
    expect(locked.first.key, 'Bm');
    // Deck B holds no track: nothing to compare against.
    expect(locked.last.bpm, isNull);
    expect(locked.last.key, isNull);

    // Without key lock the tempo ratio moves the pitch too: speed 0.72 is
    // 0.9648, about -0.6 semitones, which rounds to -1 and lands on G#m.
    final unlocked = containerFor(
      _snapshot(keyLock: false, speed: 0.72, keyShift: 0),
    ).read(deckReferencesProvider);
    expect(unlocked.first.key, 'G#m');
    final stillLocked = containerFor(_snapshot(speed: 0.72, keyShift: 0))
        .read(deckReferencesProvider);
    expect(stillLocked.first.key, 'Am');
  });

  testWidgets('debounces rapid reference changes into one push and a reload', (
    tester,
  ) async {
    final transport = FakeLibraryTransport(collections: const [_smart]);
    addTearDown(transport.dispose);
    final engine = _FakeEngineUi(_snapshot());
    var loads = 0;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => transport),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          engineUiProvider.overrideWith(() => engine),
          deckLibraryTrackProvider.overrideWith((ref, deckId) => _track),
          activeCollectionIsSmartProvider.overrideWithValue(true),
          collectionTracksProvider.overrideWith(
            () => SeededCollectionTracks((ref) async {
              loads++;
              return const [];
            }),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const _BridgeProbe(),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // The first reference is seeded immediately, so the library is never stale.
    expect(transport.setDeckReferenceCalls, hasLength(1));
    // The seeded list loads once for the null active id and once when the
    // collections resolve.
    final baseline = loads;

    for (var step = 1; step <= 5; step++) {
      engine.set(_snapshot(speed: 0.55 + step * 0.002));
      await tester.pump(const Duration(milliseconds: 20));
    }
    // Still inside the debounce window: nothing reached the library yet.
    expect(transport.setDeckReferenceCalls, hasLength(1));

    await tester.pump(kDeckReferenceDebounce);
    await tester.pumpAndSettle();

    expect(transport.setDeckReferenceCalls, hasLength(2));
    final pushed = transport.setDeckReferenceCalls.last;
    expect(pushed.first.deckId, 0);
    // The last fader position wins: speed 0.56 → 128 × 0.9904.
    expect(pushed.first.bpm, closeTo(128 * 0.9904, 1e-9));
    expect(pushed.first.key, 'Bm');
    expect(loads, baseline + 1, reason: 'one reload for the debounced change');
  });

  testWidgets('returning the fader cancels the pending push', (tester) async {
    final transport = FakeLibraryTransport(collections: const [_smart]);
    addTearDown(transport.dispose);
    final engine = _FakeEngineUi(_snapshot());
    var loads = 0;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => transport),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          engineUiProvider.overrideWith(() => engine),
          deckLibraryTrackProvider.overrideWith((ref, deckId) => _track),
          activeCollectionIsSmartProvider.overrideWithValue(true),
          collectionTracksProvider.overrideWith(
            () => SeededCollectionTracks((ref) async {
              loads++;
              return const [];
            }),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const _BridgeProbe(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(transport.setDeckReferenceCalls, hasLength(1));
    final baseline = loads;

    // Move the fader, then put it back before the debounce fires.
    engine.set(_snapshot(speed: 0.56));
    await tester.pump(const Duration(milliseconds: 50));
    engine.set(_snapshot());
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(
      transport.setDeckReferenceCalls,
      hasLength(1),
      reason: 'the intermediate position must not be pushed later',
    );
    expect(loads, baseline);
  });

  testWidgets('an unchanged reference does not push again', (tester) async {
    final transport = FakeLibraryTransport(collections: const [_smart]);
    addTearDown(transport.dispose);
    final engine = _FakeEngineUi(_snapshot());
    var loads = 0;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => transport),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          engineUiProvider.overrideWith(() => engine),
          deckLibraryTrackProvider.overrideWith((ref, deckId) => _track),
          activeCollectionIsSmartProvider.overrideWithValue(true),
          collectionTracksProvider.overrideWith(
            () => SeededCollectionTracks((ref) async {
              loads++;
              return const [];
            }),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const _BridgeProbe(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(transport.setDeckReferenceCalls, hasLength(1));
    final baseline = loads;

    // A redraw that does not move the fader keeps the same sounding values.
    engine.set(_snapshot());
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(transport.setDeckReferenceCalls, hasLength(1));
    expect(loads, baseline);
  });

  test('the active collection is smart only for smart playlists', () async {
    Future<bool> resolvedKind(
      List<LibraryCollectionSummary> collections,
    ) async {
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async => collections),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(collectionsProvider, (_, _) {});
      await container.read(collectionsProvider.future);
      final isSmart = container.read(activeCollectionIsSmartProvider);
      sub.close();
      return isSmart;
    }

    expect(await resolvedKind(const [_smart, _folder]), isTrue);
    expect(await resolvedKind(const [_folder]), isFalse);
  });
}
