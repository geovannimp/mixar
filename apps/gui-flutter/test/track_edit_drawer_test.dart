import 'dart:async';

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

  Future<FakeLibraryTransport> pumpDrawer(
    WidgetTester tester, [
    FakeLibraryTransport? transport,
  ]) async {
    final fake =
        transport ??
        FakeLibraryTransport(getTrackHandler: (id) async => _track);
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
  testWidgets('an untouched invalid prefill does not block other edits', (
    tester,
  ) async {
    // A track stored with a bogus BPM (e.g. a malformed file tag) must not
    // stop the user from editing the title.
    final fake = FakeLibraryTransport(
      getTrackHandler: (id) async => const LibraryTrackSummary(
        id: 't1',
        displayName: 'Track',
        title: 'Track',
        bpm: -4,
        path: '/tmp/t1.wav',
      ),
    );
    await pumpDrawer(tester, fake);

    expect(
      tester.widget<EditableText>(fieldFor('BPM')).controller.text,
      '-4',
      reason: 'pre-existing invalid value is shown as-is',
    );

    await tester.enterText(fieldFor('Title'), 'Edited');
    await tester.pump();
    expect(saveEnabled(tester), isTrue);

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    final call = fake.updateTrackMetadataCalls.single;
    expect(call.title, 'Edited');
    // The untouched invalid BPM is sent back as stored: neither cleared nor
    // allowed to fail the save (the store accepts an unchanged value).
    expect(call.bpm, -4);
    expect(find.text('Save'), findsNothing, reason: 'drawer popped on save');
  });

  testWidgets('untouched fields round-trip verbatim', (tester) async {
    // Stored 124.035 must show as 124.035 (not a rounded form) so an edit to
    // the title can never commit a BPM the user never saw.
    final fake = FakeLibraryTransport(
      getTrackHandler: (id) async => const LibraryTrackSummary(
        id: 't1',
        displayName: 'Track',
        title: 'Track',
        artist: 'Art',
        genre: 'House',
        bpm: 124.035,
        key: 'Am',
        isrc: 'OLD1',
        path: '/tmp/t1.wav',
      ),
    );
    await pumpDrawer(tester, fake);

    expect(
      tester.widget<EditableText>(fieldFor('BPM')).controller.text,
      '124.035',
    );

    await tester.enterText(fieldFor('Title'), 'Edited');
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    final call = fake.updateTrackMetadataCalls.single;
    expect(call.title, 'Edited');
    expect(call.bpm, 124.035, reason: 'stored precision kept');
    expect(call.album, isNull, reason: 'a stored null stays null');
    expect(call.genre, 'House');
    expect(call.key, 'Am');
    expect(call.isrc, 'OLD1');
    expect(call.artist, 'Art', reason: 'untouched non-null round-trips');
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

    // Derive the barrier point from the drawer's own geometry: half way to
    // whatever the panel's left edge actually is, so a width/layout change
    // moves the tap with it instead of leaving a magic coordinate behind.
    final saveTopLeft = tester.getTopLeft(find.text('Save'));
    await tester.tapAt(Offset(saveTopLeft.dx / 2, saveTopLeft.dy));
    await tester.pumpAndSettle();

    expect(fake.updateTrackMetadataCalls, isEmpty);
    expect(find.text('Save'), findsNothing, reason: 'drawer closed');
  });

  testWidgets('a transport failure reports and keeps the drawer open', (
    tester,
  ) async {
    final fake = await pumpDrawer(tester);
    fake.updateTrackMetadataHandler = (call) async =>
        throw StateError('backend down');

    await tester.enterText(fieldFor('Title'), 'Edited');
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(find.text('Save'), findsOneWidget, reason: 'drawer stays open');
    final container = ProviderScope.containerOf(
      tester.element(find.text('Save')),
    );
    expect(
      container.read(appNoticeProvider)?.message,
      contains('backend down'),
      reason: 'failure surfaces through appNotice (AppNoticeBridge toasts it)',
    );
    // `saving` is cleared in finally, so the user can retry.
    expect(saveEnabled(tester), isTrue, reason: 'saving reset after failure');
    expect(fake.updateTrackMetadataCalls, hasLength(1));
  });

  testWidgets('a track that vanished reports and opens no drawer', (
    tester,
  ) async {
    final fake = FakeLibraryTransport(getTrackHandler: (id) async => null);
    await pumpDrawer(tester, fake);

    expect(find.text('Save'), findsNothing, reason: 'no drawer without a row');
    final container = ProviderScope.containerOf(
      tester.element(find.text('open')),
    );
    expect(
      container.read(appNoticeProvider)?.message,
      'Track no longer exists in the library',
    );
    expect(fake.getTrackCalls, ['t1'], reason: 'asked for the tapped row');
  });

  testWidgets('a row dropped before saving fails instead of pretending', (
    tester,
  ) async {
    // Open on a real row, then let a re-scan drop it before Save runs: the
    // store would no-op for an unknown id, so the save must fail loudly
    // rather than close the drawer and lose the edit.
    var reads = 0;
    final fake = FakeLibraryTransport(
      getTrackHandler: (id) async => ++reads == 1 ? _track : null,
    );
    await pumpDrawer(tester, fake);

    await tester.enterText(fieldFor('Title'), 'Edited');
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(fake.updateTrackMetadataCalls, isEmpty, reason: 'no phantom write');
    expect(find.text('Save'), findsOneWidget, reason: 'drawer stays open');
    final container = ProviderScope.containerOf(
      tester.element(find.text('Save')),
    );
    expect(
      container.read(appNoticeProvider)?.message,
      contains('Track no longer exists'),
    );
    expect(saveEnabled(tester), isTrue, reason: 'saving reset, retry possible');
  });

  testWidgets('dismissing during the save re-read still persists the edit', (
    tester,
  ) async {
    // Tap Save, then dismiss while the re-read is in flight. `dismissed` only
    // flips on disposal (useEffect cleanup), so the pop is run to completion
    // before the gate opens — otherwise the test would pass even with a
    // pre-write `if (dismissed.value) return;` guard, i.e. it would not cover
    // the regression it exists for.
    var reads = 0;
    final gate = Completer<void>();
    final fake = FakeLibraryTransport(
      getTrackHandler: (id) async {
        reads++;
        if (reads == 2) {
          await gate.future;
        }
        return _track;
      },
    );
    await pumpDrawer(tester, fake);

    await tester.enterText(fieldFor('Title'), 'Edited');
    await tester.pump();
    await tester.tap(find.text('Save'));
    await tester.pump(); // Save is in flight, blocked on the gate
    expect(fake.updateTrackMetadataCalls, isEmpty);

    await tester.tap(find.bySemanticsLabel('Close'));
    // Finish the pop transition so the panel is disposed and `dismissed` is
    // true when the re-read resumes.
    await tester.pumpAndSettle();
    expect(find.text('Save'), findsNothing, reason: 'panel disposed');

    gate.complete();
    await tester.pumpAndSettle();

    expect(
      fake.updateTrackMetadataCalls.single.title,
      'Edited',
      reason: 'the requested save is not abandoned by dismissal',
    );
  });

  testWidgets('clearing a field sends blank so the column is cleared', (
    tester,
  ) async {
    final fake = await pumpDrawer(tester);

    expect(
      tester.widget<EditableText>(fieldFor('Album')).controller.text,
      'Album',
    );
    await tester.enterText(fieldFor('Album'), '');
    await tester.pump();
    expect(saveEnabled(tester), isTrue, reason: 'clearing is a change');

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    final call = fake.updateTrackMetadataCalls.single;
    expect(
      call.album,
      '',
      reason: 'blank reaches the store, which clears the column',
    );
    expect(call.title, 'Track', reason: 'other fields stay as loaded');
  });
}
