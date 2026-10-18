import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/track_edit_drawer.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

const _track = LibraryTrackSummary(
  id: 't1',
  displayName: 'Track',
  title: 'Track',
  artist: 'Artist',
  album: 'Album',
  genre: 'House',
  bpm: 128,
  key: 'Am',
  isrc: 'USRC1',
  path: '/tmp/t1.wav',
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<FakeLibraryTransport> pumpDrawer(WidgetTester tester) async {
    final fake = FakeLibraryTransport(getTrackHandler: (id) async => _track);
    addTearDown(fake.dispose);
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => fake),
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(defaultAppSettings()),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: Consumer(
            builder: (context, ref, _) => Center(
              child: GestureDetector(
                onTap: () =>
                    showTrackEditDrawer(context, ref, trackId: _track.id),
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    return fake;
  }

  Finder saveButton() =>
      find.ancestor(of: find.text('Save'), matching: find.byType(AppButton));

  bool saveEnabled(WidgetTester tester) =>
      tester.widget<AppButton>(saveButton()).onPress != null;

  Finder fieldFor(String label) => find.descendant(
    of: find.widgetWithText(MixarInput, label),
    matching: find.byType(EditableText),
  );

  testWidgets('drawer prefills and saves all seven fields', (tester) async {
    final fake = await pumpDrawer(tester);

    for (final label in [
      'Title',
      'Artist',
      'Album',
      'Genre',
      'BPM',
      'Key',
      'ISRC',
    ]) {
      expect(fieldFor(label), findsOneWidget, reason: '$label field rendered');
    }
    expect(tester.widget<EditableText>(fieldFor('BPM')).controller.text, '128');

    await tester.enterText(fieldFor('Title'), 'Edited Title');
    await tester.enterText(fieldFor('Artist'), 'Edited Artist');
    await tester.enterText(fieldFor('Album'), 'Edited Album');
    await tester.enterText(fieldFor('Genre'), 'Techno');
    await tester.enterText(fieldFor('BPM'), '128.5');
    await tester.enterText(fieldFor('Key'), '8A');
    await tester.enterText(fieldFor('ISRC'), 'NEW1');
    await tester.pump();

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    final call = fake.updateTrackMetadataCalls.single;
    expect(call.trackId, 't1');
    expect(call.title, 'Edited Title');
    expect(call.artist, 'Edited Artist');
    expect(call.album, 'Edited Album');
    expect(call.genre, 'Techno');
    expect(call.bpm, 128.5);
    // Raw strings: trimming / Camelot normalization happens Rust-side.
    expect(call.key, '8A');
    expect(call.isrc, 'NEW1');
    expect(find.text('Save'), findsNothing, reason: 'drawer popped on save');
  });

  testWidgets('Save is disabled until a field changes', (tester) async {
    await pumpDrawer(tester);

    expect(saveEnabled(tester), isFalse, reason: 'no edits yet');

    await tester.enterText(fieldFor('Title'), 'Edited Title');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    // Reverting to the loaded value disables it again.
    await tester.enterText(fieldFor('Title'), 'Track');
    await tester.pump();
    expect(saveEnabled(tester), isFalse);
  });

  testWidgets('reverting a field disables Save again', (tester) async {
    await pumpDrawer(tester);

    await tester.enterText(fieldFor('Artist'), '  Artist  ');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    // Back to the loaded value: nothing left to persist.
    await tester.enterText(fieldFor('Artist'), 'Artist');
    await tester.pump();
    expect(saveEnabled(tester), isFalse);
  });

  testWidgets('Save spans the drawer footer', (tester) async {
    await pumpDrawer(tester);

    final save = tester.getSize(saveButton());
    expect(
      save.width,
      moreOrLessEquals(trackEditDrawerWidth - 32, epsilon: 0.5),
      reason: 'Save fills the drawer width minus its 16px side insets',
    );
  });

  testWidgets('invalid BPM shows an inline error and blocks save', (
    tester,
  ) async {
    final fake = await pumpDrawer(tester);

    await tester.enterText(fieldFor('BPM'), 'abc');
    // A frame must pass for Save to pick up the edit, as in real use.
    await tester.pump();
    expect(saveEnabled(tester), isTrue, reason: 'unparseable BPM is a change');
    expect(
      find.text('BPM must be a positive number'),
      findsOneWidget,
      reason: 'validated on interaction, before Save is pressed',
    );

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(fake.updateTrackMetadataCalls, isEmpty);
    expect(find.text('Save'), findsOneWidget, reason: 'drawer stays open');
    final container = ProviderScope.containerOf(
      tester.element(find.text('Save')),
    );
    expect(
      container.read(appNoticeProvider)?.message,
      'Fix the highlighted fields',
    );
  });

  testWidgets('an invalid key is reported inline', (tester) async {
    final fake = await pumpDrawer(tester);

    await tester.enterText(fieldFor('Key'), 'nonsense');
    await tester.pump();
    expect(
      find.text('Use a musical key (Am) or Camelot code (8A)'),
      findsOneWidget,
    );

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();
    expect(fake.updateTrackMetadataCalls, isEmpty, reason: 'save blocked');

    // A Camelot code the Rust path can normalize passes validation.
    await tester.enterText(fieldFor('Key'), '8A');
    await tester.pump();
    expect(
      find.text('Use a musical key (Am) or Camelot code (8A)'),
      findsNothing,
    );
  });

  testWidgets('dismissing the drawer does not save', (tester) async {
    final fake = await pumpDrawer(tester);

    await tester.enterText(fieldFor('Title'), 'Discarded');
    await tester.pump();

    await tester.tap(find.bySemanticsLabel('Close'));
    await tester.pumpAndSettle();

    expect(fake.updateTrackMetadataCalls, isEmpty);
    expect(find.text('Save'), findsNothing, reason: 'drawer closed');
  });

  testWidgets('tapping the barrier closes without saving', (tester) async {
    final fake = await pumpDrawer(tester);

    await tester.enterText(fieldFor('Title'), 'Discarded');
    await tester.pump();

    // Left of the 420px right-edge drawer.
    await tester.tapAt(const Offset(40, 300));
    await tester.pumpAndSettle();

    expect(fake.updateTrackMetadataCalls, isEmpty);
    expect(find.text('Save'), findsNothing, reason: 'drawer closed');
  });
}
