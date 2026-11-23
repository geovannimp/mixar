import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show EngineTransport;
import 'package:gui_flutter/widgets/mixer/deck_cue_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _LoadedEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => const EngineUiSnapshot(
    running: true,
    trackPaths: {0: '/song.wav'},
    durationMs: {0: 205000},
  );
}

class _CueTransport extends FakeEngineTransport {
  final Completer<void> begin = Completer<void>();
  int endCalls = 0;
  bool failFirstEnd = true;

  @override
  Future<void> beginCueHold({required int deckId}) => begin.future;

  @override
  Future<void> endCueHold({required int deckId}) async {
    endCalls++;
    if (failFirstEnd) {
      failFirstEnd = false;
      throw StateError('end failed');
    }
  }
}

class _RecordingCueTransport extends FakeEngineTransport {
  int begins = 0;
  int ends = 0;

  @override
  Future<void> beginCueHold({required int deckId}) async {
    begins++;
  }

  @override
  Future<void> endCueHold({required int deckId}) async {
    ends++;
  }
}

class _TransportSwitch extends Notifier<EngineTransport?> {
  @override
  EngineTransport? build() => null;

  void set(EngineTransport? value) => state = value;
}

final _switchProvider = NotifierProvider<_TransportSwitch, EngineTransport?>(
  _TransportSwitch.new,
);

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    required EngineTransport? Function(Ref ref) engine,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    tester.view.physicalSize = const Size(370, 560);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(_LoadedEngineUi.new),
          engineTransportProvider.overrideWith((ref) async => engine(ref)),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: DeckPanel(deckId: 0, label: 'DECK A', accent: DeckAccent.a),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'an unmount during begin releases once and retries a failed end',
    (tester) async {
      final transport = _CueTransport();
      await pumpPanel(tester, engine: (_) => transport);

      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(DeckCueButton)),
      );
      await tester.pump(const Duration(milliseconds: 200));

      // Unmount while the engine has not answered the hold yet.
      await tester.pumpWidget(const SizedBox());
      transport.begin.complete();
      for (var i = 0; i < 5; i++) {
        await tester.pump();
      }

      expect(
        transport.endCalls,
        2,
        reason: 'the failed release is retried once during dispose',
      );

      await gesture.up();
      await tester.pump();
    },
  );

  testWidgets('a hold releases to the transport that began it', (tester) async {
    await pumpPanel(tester, engine: (ref) => ref.watch(_switchProvider));
    final container = ProviderScope.containerOf(
      tester.element(find.byType(DeckPanel)),
    );
    final oldTransport = _RecordingCueTransport();
    final newTransport = _RecordingCueTransport();
    container.read(_switchProvider.notifier).set(oldTransport);
    await tester.pumpAndSettle();

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(DeckCueButton)),
    );
    await tester.pump(const Duration(milliseconds: 200));
    expect(oldTransport.begins, 1);

    // A replacement transport lands while the hold is still down.
    container.read(_switchProvider.notifier).set(newTransport);
    await tester.pumpAndSettle();

    await gesture.up();
    await tester.pumpAndSettle();

    expect(oldTransport.ends, 1);
    expect(newTransport.begins, 0);
    expect(newTransport.ends, 0);
  });
}
