import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

void main() {
  test('deckTrackTitleProvider prefers library title over path stem', () {
    const table = LibraryTrackSummary(
      id: 'a',
      displayName: 'Display',
      title: 'Library Title',
      path: '/music/file.flac',
    );
    final container = ProviderContainer(
      overrides: [
        deckTrackIdProvider(0).overrideWith((ref) => 'a'),
        libraryTableTracksProvider.overrideWith(
          (ref) => const AsyncData([table]),
        ),
        collectionTracksProvider.overrideWith(
          (ref) async => const <LibraryTrackSummary>[],
        ),
        driveResolvedByPathProvider.overrideWith(
          (ref) async => const <String, LibraryTrackSummary>{},
        ),
        libraryTrackByIdProvider('a').overrideWith((ref) async => null),
        engineUiProvider.overrideWith(_SeedEngineUi.new),
      ],
    );
    addTearDown(container.dispose);
    expect(container.read(deckTrackTitleProvider(0)), 'Library Title');
  });

  test('deckTrackTitleProvider falls back to file stem from engine path', () {
    final container = ProviderContainer(
      overrides: [
        deckTrackIdProvider(0).overrideWith((ref) => null),
        libraryTableTracksProvider.overrideWith(
          (ref) => const AsyncData(<LibraryTrackSummary>[]),
        ),
        collectionTracksProvider.overrideWith(
          (ref) async => const <LibraryTrackSummary>[],
        ),
        driveResolvedByPathProvider.overrideWith(
          (ref) async => const <String, LibraryTrackSummary>{},
        ),
        engineUiProvider.overrideWith(_SeedEngineUi.new),
      ],
    );
    addTearDown(container.dispose);
    expect(container.read(deckTrackTitleProvider(0)), 'file');
    expect(container.read(deckHasTrackProvider(0)), isTrue);
  });

  test('deckHasTrackProvider is true with duration and no title path', () {
    final container = ProviderContainer(
      overrides: [engineUiProvider.overrideWith(_DurationOnlyEngineUi.new)],
    );
    addTearDown(container.dispose);
    expect(container.read(deckHasTrackProvider(0)), isTrue);
    expect(container.read(deckTrackTitleProvider(0)), isNull);
  });
}

class _SeedEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvt(
    EngineUiSnapshot.empty,
    const EngineEvt(
      kind: EngineEvtKind.updated,
      deckId: 0,
      trackPath: '/music/file.flac',
      durationMs: 1000,
      durationKnown: true,
    ),
  );
}

class _DurationOnlyEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvt(
    EngineUiSnapshot.empty,
    const EngineEvt(
      kind: EngineEvtKind.updated,
      deckId: 0,
      durationMs: 1000,
      durationKnown: true,
    ),
  );
}
