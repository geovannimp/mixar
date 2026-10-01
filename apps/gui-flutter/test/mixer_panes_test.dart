import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/library_list_chrome.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/library_panel.dart';
import 'package:gui_flutter/mixer/mixer_page.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/m_tabs.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:panes/panes.dart';
import 'package:riverpod/src/framework.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 0,
  );

  List<Override> overrides() => [
    collectionsProvider.overrideWith((ref) async => [collection]),
    collectionTracksProvider.overrideWith((ref) async => const []),
    historySessionsProvider.overrideWith((ref) async => const []),
    historyCanResumeProvider.overrideWith((ref) async => false),
    libraryEventsBootstrapProvider.overrideWith((ref) {}),
    historySettingsBootstrapProvider.overrideWith((ref) {}),
    librarySettingsBootstrapProvider.overrideWith((ref) {}),
    appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
  ];

  Future<void> pumpSized(WidgetTester tester, {required Widget child}) async {
    final theme = MixarThemeData.dark();
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: overrides(),
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(body: child),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('LibraryPanel builds MultiPane split', (tester) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();
    expect(find.byType(MultiPane), findsOneWidget);
    expect(find.byIcon(LucideIcons.library), findsOneWidget);
  });

  testWidgets('MixerPage builds MultiPane split', (tester) async {
    await pumpSized(tester, child: const MixerPage());
    await tester.pumpAndSettle();
    expect(find.byType(MultiPane), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
  });

  testWidgets('sidebar tabs and the track toolbar share a header height', (
    tester,
  ) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();

    // Both header rows are found by their stable keys, not by a fill colour
    // that `muted` and `secondary` share.
    final tabBar = find.byKey(kMTabBarKey);
    final toolbar = find.byKey(kLibraryToolbarKey);

    expect(
      tester.getSize(toolbar).height,
      tester.getSize(tabBar).height,
      reason: 'the topbar and the tabs must share one header height',
    );

    // The filter field's band fills the header up to the bottom border, so
    // there is no gap above the field and it does not paint over the border.
    final band = find.byWidgetPredicate(
      (w) =>
          w is ColoredBox &&
          w.color == libraryToolbarFieldColor(MixarThemeData.dark()),
    );
    expect(
      tester.getSize(band).height,
      tester.getSize(toolbar).height - MixarThemeData.dark().style.borderWidth,
      reason: 'the field band must fill the header up to the bottom border',
    );

    // Both header rows draw the same bottom border, so the rule reads as one
    // continuous line across the split.
    Border bottom(DecoratedBox box) =>
        (box.decoration as BoxDecoration).border! as Border;
    expect(
      bottom(tester.widget<DecoratedBox>(tabBar)).bottom.color,
      bottom(tester.widget<DecoratedBox>(toolbar)).bottom.color,
      reason: 'the tabs and topbar bottom borders must match',
    );
  });
}
