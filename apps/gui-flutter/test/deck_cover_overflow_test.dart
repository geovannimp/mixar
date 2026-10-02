import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/mixer/deck_track_info.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

class _SeededEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvt(
    EngineUiSnapshot.empty,
    const EngineEvt(
      kind: EngineEvtKind.updated,
      deckId: 0,
      trackPath: '/t.flac',
      trackId: 't1',
      durationMs: 156000,
    ),
  );
}

class _FakeArtwork extends ArtworkCache {
  new(this.bytes);
  final Uint8List bytes;
  @override
  Map<String, Uint8List?> build() => {'t1': bytes};
  @override
  Future<void> ensureLoaded(List<String> ids) async {}
}

/// Real, large cover (1500x1500) — the original overflow trigger.
Future<Uint8List> _png(int w, int h) async {
  final recorder = ui.PictureRecorder();
  ui.Canvas(recorder).drawRect(
    ui.Rect.fromLTWH(0, 0, w.toDouble(), h.toDouble()),
    ui.Paint()..color = const ui.Color(0xFFFF0000),
  );
  final image = await recorder.endRecording().toImage(w, h);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  return data!.buffer.asUint8List();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('large decoded cover does not overflow the deck card', (
    tester,
  ) async {
    final bytes = (await tester.runAsync(() => _png(1500, 1500)))!;

    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(_SeededEngineUi.new),
          artworkCacheProvider.overrideWith(() => _FakeArtwork(bytes)),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          deckLibraryTrackProvider.overrideWith(
            (ref, deckId) => const LibraryTrackSummary(
              id: 't1',
              displayName: 'Amplified Space',
              title: 'Amplified Space',
              artist: 'Native Instruments',
              key: '5A',
              path: '/t.flac',
            ),
          ),
          deckSkeletonProvider.overrideWith((ref, deckId) => false),
          deckHotCuesProvider.overrideWith((ref, deckId) => const []),
          deckSavedLoopsProvider.overrideWith((ref, deckId) => const []),
          waveformOverviewProvider.overrideWith(
            (ref, id) async => const <SpectralPeak>[],
          ),
          beatGridFetchProvider.overrideWith((ref, id) async => null),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(
            body: Center(
              child: SizedBox(
                width: 500,
                height: 390,
                child: DeckTrackInfo(
                  deckId: 0,
                  hasTrack: true,
                  title: 'Amplified Space',
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    // Force the cover to decode so its intrinsic size participates in layout.
    await tester.runAsync(() async {
      await precacheImage(
        MemoryImage(bytes),
        tester.element(find.byType(DeckTrackInfo)),
      );
    });
    await tester.pumpAndSettle();

    // No RenderFlex overflow (the bug) and the card fits the deck slot.
    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byType(DeckTrackInfo)).height,
      lessThanOrEqualTo(390),
    );
  });
}
