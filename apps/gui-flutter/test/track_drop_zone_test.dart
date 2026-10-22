import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/widgets/mixer/track_drop_zone.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

import 'support/fake_engine_transport.dart';

/// Records the tracks a deck load asked for.
class _RecordingDeckLoadTransport implements DeckLoadTransport {
  final List<(int deckId, String trackId)> libraryLoads = [];
  final List<(int deckId, String path)> pathLoads = [];

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    libraryLoads.add((deckId, trackId));
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    pathLoads.add((deckId, path));
  }
}

const _sourceKey = Key('drag-source');
const _targetKey = Key('drop-target');

void main() {
  testWidgets('a track dragged onto a deck loads into that deck', (
    tester,
  ) async {
    final loads = _RecordingDeckLoadTransport();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineTransportProvider.overrideWith(
            (ref) async => FakeEngineTransport(),
          ),
          engineRunningProvider.overrideWithValue(true),
          deckLoadTransportProvider.overrideWith((ref) async => loads),
        ],
        child: MixarTheme(
          data: MixarThemeData.light(),
          child: MaterialApp(
            home: Scaffold(
              body: Stack(
                children: [
                  Positioned(
                    top: 20,
                    left: 20,
                    child: libraryTrackDragSource(
                      payload: const TrackDragPayload(
                        source: TrackDragSource.library,
                        trackId: 't1',
                        path: '/lib/a.wav',
                        title: 'Alpha',
                      ),
                      child: const ColoredBox(
                        key: _sourceKey,
                        color: Color(0xFF000000),
                        child: SizedBox(width: 40, height: 40),
                      ),
                    ),
                  ),
                  const Positioned(
                    bottom: 40,
                    left: 20,
                    child: TrackDropZone(
                      deckId: 1,
                      child: ColoredBox(
                        key: _targetKey,
                        color: Color(0xFF00FF00),
                        child: SizedBox(width: 120, height: 120),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final gesture = await tester.startGesture(
      tester.getCenter(find.byKey(_sourceKey)),
    );
    await gesture.moveTo(tester.getCenter(find.byKey(_targetKey)));
    await tester.pump();
    await gesture.up();
    await tester.pumpAndSettle();

    expect(loads.libraryLoads, [(1, 't1')]);
    expect(loads.pathLoads, isEmpty);
  });

  testWidgets('a track dragged onto a sampler pad is assigned to it', (
    tester,
  ) async {
    final assigned = <(int slot, TrackDragPayload)>[];
    await tester.pumpWidget(
      ProviderScope(
        child: MixarTheme(
          data: MixarThemeData.light(),
          child: MaterialApp(
            home: Scaffold(
              body: Stack(
                children: [
                  Positioned(
                    top: 20,
                    left: 20,
                    child: libraryTrackDragSource(
                      payload: const TrackDragPayload(
                        source: TrackDragSource.library,
                        trackId: 't1',
                        path: '/lib/a.wav',
                        title: 'Alpha',
                      ),
                      child: const ColoredBox(
                        key: _sourceKey,
                        color: Color(0xFF000000),
                        child: SizedBox(width: 40, height: 40),
                      ),
                    ),
                  ),
                  Positioned(
                    bottom: 40,
                    left: 20,
                    width: 320,
                    height: 200,
                    child: SamplerPads(
                      slots: const [],
                      holdLike: false,
                      onPress: (_, _) {},
                      onRelease: (_) {},
                      onAssign: (slot, payload) =>
                          assigned.add((slot, payload)),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final gesture = await tester.startGesture(
      tester.getCenter(find.byKey(_sourceKey)),
    );
    await gesture.moveTo(tester.getCenter(find.byType(PadFace).first));
    await tester.pump();
    await gesture.up();
    await tester.pumpAndSettle();

    expect(assigned, hasLength(1));
    expect(assigned.single.$1, 0);
    expect(assigned.single.$2.trackId, 't1');
  });
}
