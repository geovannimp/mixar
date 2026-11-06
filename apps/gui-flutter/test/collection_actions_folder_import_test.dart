import 'package:flutter/foundation.dart'
    show TargetPlatform, debugDefaultTargetPlatformOverride;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/android_media_access.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';

import 'support/fake_library_transport.dart';

/// Folder import recorder: the real scan is host-side, so the fake reports the
/// counts the guard and the notice read.
class _FolderTransport extends FakeLibraryTransport {
  int folderCalls = 0;
  int added = 1;
  int skipped = 0;
  int failed = 0;

  @override
  Future<AddFolderCollectionResult> addFolderCollection({
    required String folderPath,
    required bool scanFolderTree,
    String? name,
  }) async {
    folderCalls++;
    return AddFolderCollectionResult(
      collection: LibraryCollectionSummary(
        id: 'folder:$folderPath',
        name: name ?? 'Music',
        kind: 'folder',
        path: folderPath,
        trackCount: added,
      ),
      added: added,
      updated: 0,
      skipped: skipped,
      failed: failed,
    );
  }
}

class _RefProbe extends ConsumerWidget {
  const new({required this.onRef});

  final void Function(WidgetRef ref) onRef;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    onRef(ref);
    return const SizedBox.shrink();
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final channel = MethodChannel(androidMediaAccessChannel);
  const request = CreateCollectionResult(
    name: 'Music',
    type: CreateCollectionType.folder,
    folderPath: '/storage/emulated/0/Music',
    scanSubfolders: true,
    sortable: true,
  );

  late _FolderTransport transport;
  late WidgetRef ref;

  void mockPermission({required bool granted}) {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async => granted);
  }

  AppNotice? notice(WidgetTester tester) =>
      ProviderScope.containerOf(tester.element(find.byType(_RefProbe)))
          .read(appNoticeProvider);

  void expectErrorNotice(WidgetTester tester, String message) {
    expect(notice(tester)?.kind, AppNoticeKind.error);
    expect(notice(tester)?.message, message);
  }

  void expectImportedFolder(WidgetTester tester) {
    expect(transport.folderCalls, 1);
    expect(notice(tester), isNull);
  }

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  /// Runs one folder import end to end: platform, permission answer, widget
  /// tree and the action itself.
  Future<LibraryCollectionSummary?> importFolder(
    WidgetTester tester, {
    TargetPlatform platform = TargetPlatform.android,
    bool granted = true,
    int added = 1,
    int skipped = 0,
    int failed = 0,
  }) async {
    transport = _FolderTransport()
      ..added = added
      ..skipped = skipped
      ..failed = failed;
    addTearDown(transport.dispose);
    debugDefaultTargetPlatformOverride = platform;
    try {
      mockPermission(granted: granted);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryTransportProvider.overrideWith(
              (providerRef) async => transport,
            ),
          ],
          child: _RefProbe(onRef: (probe) => ref = probe),
        ),
      );
      return await createCollection(ref, request);
    } finally {
      // Cleared before the binding's end-of-test invariant check runs; a
      // tearDown is too late for that check.
      debugDefaultTargetPlatformOverride = null;
    }
  }

  testWidgets('refused media permission stops the import before the scan', (
    tester,
  ) async {
    final collection = await importFolder(tester, granted: false);

    expect(collection, isNull);
    expect(transport.folderCalls, 0);
    expectErrorNotice(tester, audioReadDeniedMessage);
  });

  testWidgets('granted permission imports the folder', (tester) async {
    final collection = await importFolder(tester);

    expect(collection?.name, 'Music');
    expectImportedFolder(tester);
  });

  testWidgets('a folder with no audio files says so and is not selected', (
    tester,
  ) async {
    final collection = await importFolder(tester, added: 0);

    expect(collection, isNull);
    expect(notice(tester)?.kind, AppNoticeKind.notice);
    expect(notice(tester)?.message, 'No audio files found in “Music”.');
  });

  testWidgets('skipped files are not reported as an empty folder', (
    tester,
  ) async {
    final collection = await importFolder(tester, added: 0, skipped: 2);

    expect(collection, isNotNull);
    expect(notice(tester), isNull);
  });

  testWidgets('unreadable files are named, not reported as an empty folder', (
    tester,
  ) async {
    final collection = await importFolder(tester, added: 0, failed: 1);

    expect(collection, isNull);
    expectErrorNotice(tester, 'Could not read 1 file in “Music”.');
  });

  testWidgets('a partial import still names the unreadable files', (
    tester,
  ) async {
    final collection = await importFolder(tester, added: 2, failed: 2);

    expect(collection, isNotNull);
    expectErrorNotice(tester, 'Could not read 2 files in “Music”.');
  });

  testWidgets('desktop does not ask for the Android permission', (
    tester,
  ) async {
    final collection = await importFolder(
      tester,
      platform: TargetPlatform.linux,
    );

    expect(collection?.name, 'Music');
    expectImportedFolder(tester);
  });
}
