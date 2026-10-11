import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'support/fake_library_transport.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  ({ProviderContainer container, FakeLibraryTransport fake}) boot(
    FakeLibraryTransport fake,
  ) {
    final container = ProviderContainer(
      overrides: [libraryTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(() {
      container.dispose();
      fake.dispose();
    });
    return (container: container, fake: fake);
  }

  test('ensureLoaded stores artwork bytes from getTrack', () async {
    final bytes = Uint8List.fromList([1, 2, 3]);
    final env = boot(
      FakeLibraryTransport(
        getTrackHandler: (id) async => LibraryTrackSummary(
          id: id,
          displayName: id,
          path: '/$id.flac',
          artwork: bytes,
        ),
      ),
    );

    await env.container.read(artworkCacheProvider.notifier).ensureLoaded(['a']);

    expect(env.container.read(artworkCacheProvider)['a'], bytes);
    expect(env.fake.getTrackCalls, ['a']);
  });

  test('ensureLoaded([]) is a no-op with no transport calls', () async {
    final env = boot(FakeLibraryTransport());
    final before = Map<String, Uint8List?>.of(
      env.container.read(artworkCacheProvider),
    );

    await env.container.read(artworkCacheProvider.notifier).ensureLoaded([]);

    expect(env.fake.getTrackCalls, isEmpty);
    expect(env.container.read(artworkCacheProvider), before);
  });

  test('ensureLoaded skips ids already present in the cache', () async {
    final env = boot(
      FakeLibraryTransport(
        getTrackHandler: (id) async => LibraryTrackSummary(
          id: id,
          displayName: id,
          path: '/$id.flac',
          artwork: Uint8List.fromList([1]),
        ),
      ),
    );
    final cache = env.container.read(artworkCacheProvider.notifier);

    await cache.ensureLoaded(['a']);
    expect(env.fake.getTrackCalls, ['a']);

    await cache.ensureLoaded(['a']);
    expect(env.fake.getTrackCalls, ['a']);
  });

  test('null artwork is cached so getTrack is not retried', () async {
    final env = boot(
      FakeLibraryTransport(
        getTrackHandler: (id) async =>
            LibraryTrackSummary(id: id, displayName: id, path: '/$id.flac'),
      ),
    );
    final cache = env.container.read(artworkCacheProvider.notifier);

    await cache.ensureLoaded(['a']);
    await cache.ensureLoaded(['a']);

    expect(env.container.read(artworkCacheProvider).containsKey('a'), isTrue);
    expect(env.container.read(artworkCacheProvider)['a'], isNull);
    expect(env.fake.getTrackCalls, ['a']);
  });

  test(
    'getTrack failure is not cached and retries on next ensureLoaded',
    () async {
      var shouldFail = true;
      final env = boot(
        FakeLibraryTransport(
          getTrackHandler: (id) async {
            if (shouldFail) {
              throw StateError('boom');
            }
            return LibraryTrackSummary(
              id: id,
              displayName: id,
              path: '/$id.flac',
              artwork: Uint8List.fromList([9]),
            );
          },
        ),
      );
      final cache = env.container.read(artworkCacheProvider.notifier);

      await cache.ensureLoaded(['a']);
      expect(
        env.container.read(artworkCacheProvider).containsKey('a'),
        isFalse,
      );

      shouldFail = false;
      await cache.ensureLoaded(['a']);
      expect(
        env.container.read(artworkCacheProvider)['a'],
        Uint8List.fromList([9]),
      );
      expect(env.fake.getTrackCalls, ['a', 'a']);
    },
  );

  test('inflight ensureLoaded dedupes getTrack for the same id', () async {
    final gate = Completer<LibraryTrackSummary?>();
    final env = boot(
      FakeLibraryTransport(getTrackHandler: (id) => gate.future),
    );
    final cache = env.container.read(artworkCacheProvider.notifier);

    final first = cache.ensureLoaded(['a']);
    await pumpEventQueue();
    final second = cache.ensureLoaded(['a']);
    expect(env.fake.getTrackCalls, ['a']);

    gate.complete(
      const LibraryTrackSummary(id: 'a', displayName: 'a', path: '/a.flac'),
    );
    await Future.wait([first, second]);
    expect(env.fake.getTrackCalls, ['a']);
  });

  test('ensureLoaded batches at most 3 concurrent getTrack calls', () async {
    final gates = <String, Completer<LibraryTrackSummary?>>{
      for (final id in ['a', 'b', 'c', 'd', 'e', 'f']) id: Completer(),
    };
    final env = boot(
      FakeLibraryTransport(getTrackHandler: (id) => gates[id]!.future),
    );

    final loading = env.container
        .read(artworkCacheProvider.notifier)
        .ensureLoaded(['a', 'b', 'c', 'd', 'e', 'f']);
    await pumpEventQueue();

    expect(env.fake.maxOutstandingGetTracks, 3);
    expect(env.fake.getTrackCalls, ['a', 'b', 'c']);

    for (final id in ['a', 'b', 'c']) {
      gates[id]!.complete(
        LibraryTrackSummary(id: id, displayName: id, path: '/$id.flac'),
      );
    }
    await pumpEventQueue();
    expect(env.fake.getTrackCalls, ['a', 'b', 'c', 'd', 'e', 'f']);
    expect(env.fake.maxOutstandingGetTracks, 3);

    for (final id in ['d', 'e', 'f']) {
      gates[id]!.complete(
        LibraryTrackSummary(id: id, displayName: id, path: '/$id.flac'),
      );
    }
    await loading;
  });

  test('eviction prefers non-wanted keys when over cap', () async {
    final env = boot(FakeLibraryTransport());
    final cache = env.container.read(artworkCacheProvider.notifier);

    final firstBatch = [for (var i = 0; i < 300; i++) 'id$i'];
    await cache.ensureLoaded(firstBatch);
    expect(env.container.read(artworkCacheProvider).length, 300);

    await cache.ensureLoaded(['fresh']);
    final map = env.container.read(artworkCacheProvider);
    expect(map.containsKey('fresh'), isTrue);
    expect(map.length, 300);
    expect(map.containsKey('id0'), isFalse);
  });
}
