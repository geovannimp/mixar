//! Create-collection dialog: smart playlists offer a rules editor and send the
//! canonical rule document to the transport.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/library/create_collection_drawer.dart';
import 'package:gui_flutter/widgets/library/library_form_drawer.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

/// The bpm 120–130 rule the editor starts a smart playlist with.
const _defaultRules =
    '{"version":1,"root":{"match":"all","rules":['
    '{"kind":"condition","field":"bpm","op":"between","min":120.0,'
    '"max":130.0}]}}';

/// Opens the dialog and can persist whatever it returned.
class _Harness extends ConsumerStatefulWidget {
  const new();

  @override
  ConsumerState<_Harness> createState() => _HarnessState();
}

class _HarnessState extends ConsumerState<_Harness> {
  CreateCollectionResult? _result;

  bool get _hasResult => _result != null;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        AppButton(
          onPress: () async {
            final result = await showCreateCollectionDrawer(context);
            if (!mounted) {
              return;
            }
            setState(() => _result = result);
          },
          child: const Text('Open'),
        ),
        AppButton(
          onPress: _hasResult
              ? () async {
                  await createCollection(ref, _result!);
                }
              : null,
          child: const Text('Persist'),
        ),
      ],
    );
  }
}

/// Pick a preset from the editor's preset list.
///
/// The list is addressed by type, not by its label: a playlist named after a
/// preset makes the label text ambiguous with the name field.
Future<void> _choosePreset(WidgetTester tester, String preset) async {
  final select = find
      .descendant(
        of: find.byType(SmartRulesEditor),
        matching: find.byType(ShadSelect<String>),
      )
      .first;
  await tester.tap(select);
  await tester.pumpAndSettle();
  await tester.tap(find.text(preset).last);
  await tester.pumpAndSettle();
}

/// Open the dialog and switch it to a smart playlist.
Future<void> _openSmartDialog(WidgetTester tester) async {
  await tester.tap(find.text('Open'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Folder').first);
  await tester.pumpAndSettle();
  await tester.tap(find.text('Smart playlist').last);
  await tester.pumpAndSettle();
}

/// The name field is the first input in the dialog body.
String _nameText(WidgetTester tester) =>
    tester.widget<ShadInput>(find.byType(ShadInput).first).controller!.text;

bool _createEnabled(WidgetTester tester) {
  final create = tester.widget<AppButton>(
    find.widgetWithText(AppButton, 'Create'),
  );
  return create.onPress != null;
}

Future<void> _pumpHarness(
  WidgetTester tester,
  FakeLibraryTransport transport,
) async {
  tester.view.physicalSize = const Size(1000, 1600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryTransportProvider.overrideWith((ref) async => transport),
        appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
      ],
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(body: Center(child: _Harness())),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a smart playlist needs a rule before it can be created', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);

    await _openSmartDialog(tester);

    expect(find.text('Smart playlist'), findsOneWidget);
    expect(find.text('Add rule'), findsOneWidget);
    expect(find.text('Add at least one rule.'), findsOneWidget);

    await tester.enterText(find.byType(ShadInput), 'Peak');
    await tester.pumpAndSettle();
    expect(_createEnabled(tester), isFalse);

    await tester.tap(find.text('Add rule'));
    await tester.pumpAndSettle();
    expect(_createEnabled(tester), isTrue);
  });

  testWidgets('the form opens as a right-edge drawer, not a dialog', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);

    expect(find.byType(LibraryFormDrawerShell), findsNothing);
    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();

    final panel = tester.getRect(find.byType(LibraryFormDrawerShell));
    expect(panel.width, createCollectionDrawerWidth);
    expect(panel.height, tester.view.physicalSize.height - kAppHeaderHeight);
    expect(panel.top, kAppHeaderHeight, reason: 'never covers the titlebar');
    expect(
      panel.right,
      tester.view.physicalSize.width,
      reason: 'flush to the right edge',
    );

    // The barrier dismisses it, reporting "nothing created".
    await tester.tapAt(const Offset(8, 8));
    await tester.pumpAndSettle();
    expect(find.byType(LibraryFormDrawerShell), findsNothing);
    expect(transport.createSmartPlaylistCalls, isEmpty);
  });

  testWidgets('clicking away from a select clears its focus ring', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);
    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();

    // The folder form has one String select: Type.
    final node = tester
        .widget<ShadSelect<String>>(find.byType(ShadSelect<String>))
        .focusNode!;
    node.requestFocus();
    await tester.pump();
    expect(node.hasFocus, isTrue);

    // A press elsewhere in the drawer, not on the control, drops the ring.
    await tester.tap(find.text('Create collection').first);
    await tester.pumpAndSettle();

    expect(node.hasFocus, isFalse);
  });

  testWidgets('a preset names an unnamed smart playlist', (tester) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);
    await _openSmartDialog(tester);
    expect(_nameText(tester), isEmpty);

    await _choosePreset(tester, 'Peak time');
    expect(_nameText(tester), 'Peak time');
    expect(_createEnabled(tester), isTrue);

    // A newer preset replaces a name this dialog derived...
    await _choosePreset(tester, 'Unrated');
    expect(_nameText(tester), 'Unrated');

    // ...but never one the user typed.
    await tester.enterText(find.byType(ShadInput).first, 'My peak');
    await tester.pumpAndSettle();
    await _choosePreset(tester, 'Peak time');
    expect(_nameText(tester), 'My peak');
  });

  testWidgets('an invalid rule document cannot be created', (tester) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);
    await _openSmartDialog(tester);

    await tester.enterText(find.byType(ShadInput), 'Peak');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Add rule'));
    await tester.pumpAndSettle();
    expect(_createEnabled(tester), isTrue);

    // An inverted window is invalid: letting it through would fail after the
    // drawer closed and lose the rule tree.
    final bounds = find.byType(ShadInput);
    await tester.enterText(bounds.at(1), '200');
    await tester.enterText(bounds.at(2), '130');
    await tester.pumpAndSettle();

    expect(_createEnabled(tester), isFalse);
    expect(find.text('min must not exceed max on bpm'), findsOneWidget);
  });

  testWidgets('clearing the preset back to the prompt blocks Create', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);
    await _openSmartDialog(tester);

    await tester.enterText(find.byType(ShadInput), 'Peak');
    await tester.pumpAndSettle();
    await _choosePreset(tester, 'Peak time');
    expect(_createEnabled(tester), isTrue);

    // The prompt is the empty document: it drops the preset's rules, and an
    // empty playlist is not something to save.
    await _choosePreset(tester, 'Choose a preset');
    expect(_createEnabled(tester), isFalse);
    expect(find.text('Add at least one rule.'), findsOneWidget);
  });

  testWidgets('creating sends the canonical rules document', (tester) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pumpHarness(tester, transport);

    await _openSmartDialog(tester);
    await tester.enterText(find.byType(ShadInput), '  Peak  ');
    await tester.pumpAndSettle();
    await tester.tap(find.text('Add rule'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Create'));
    await tester.pumpAndSettle();
    expect(find.text('Smart playlist'), findsNothing);

    await tester.tap(find.text('Persist'));
    await tester.pumpAndSettle();

    final call = transport.createSmartPlaylistCalls.single;
    expect(call.name, 'Peak');
    expect(call.rulesJson, _defaultRules);
  });
}
