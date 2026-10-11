import 'dart:async';

import 'package:flutter/foundation.dart';
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
    'getTrack failure stores null and second ensureLoaded does not refetch',
    () async {
      final env = boot(
        FakeLibraryTransport(
          getTrackHandler: (id) async {
            throw Exception('boom');
          },
        ),
      );
      final cache = env.container.read(artworkCacheProvider.notifier);

      final reported = <FlutterErrorDetails>[];
      final previous = FlutterError.onError;
      FlutterError.onError = (details) {
        previous?.call(details);
        reported.add(details);
      };
      addTearDown(() => FlutterError.onError = previous);

      await cache.ensureLoaded(['a']);
      expect(env.container.read(artworkCacheProvider).containsKey('a'), isTrue);
      expect(env.container.read(artworkCacheProvider)['a'], isNull);
      expect(env.fake.getTrackCalls, ['a']);
      expect(reported, isNotEmpty, reason: 'Exceptions must be reported');

      await cache.ensureLoaded(['a']);
      expect(env.fake.getTrackCalls, ['a'], reason: 'failed id must not retry');
    },
  );

  test('Error on one id still loads later ids and reports', () async {
    final bytes = Uint8List.fromList([9]);
    final env = boot(
      FakeLibraryTransport(
        getTrackHandler: (id) async {
          if (id == 'bad') {
            throw StateError('boom');
          }
          return LibraryTrackSummary(
            id: id,
            displayName: id,
            path: '/$id.flac',
            artwork: bytes,
          );
        },
      ),
    );
    final cache = env.container.read(artworkCacheProvider.notifier);

    final reported = <FlutterErrorDetails>[];
    final previous = FlutterError.onError;
    FlutterError.onError = (details) {
      previous?.call(details);
      reported.add(details);
    };
    addTearDown(() => FlutterError.onError = previous);

    await cache.ensureLoaded(['bad', 'ok']);
    expect(env.container.read(artworkCacheProvider)['bad'], isNull);
    expect(env.container.read(artworkCacheProvider)['ok'], bytes);
    expect(env.fake.getTrackCalls, ['bad', 'ok']);
    expect(reported, isNotEmpty, reason: 'Errors must be reported');
  });

  test('evicted failure sentinel can be refetched', () async {
    var shouldFail = true;
    final env = boot(
      FakeLibraryTransport(
        getTrackHandler: (id) async {
          if (id == 'fail' && shouldFail) {
            throw Exception('boom');
          }
          return LibraryTrackSummary(
            id: id,
            displayName: id,
            path: '/$id.flac',
            artwork: Uint8List.fromList([1]),
          );
        },
      ),
    );
    final cache = env.container.read(artworkCacheProvider.notifier);

    await cache.ensureLoaded(['fail']);
    expect(env.container.read(artworkCacheProvider)['fail'], isNull);
    expect(env.fake.getTrackCalls, ['fail']);

    // Push the sentinel out via FIFO eviction, then allow refetch.
    await cache.ensureLoaded([for (var i = 0; i < 300; i++) 'id$i']);
    expect(
      env.container.read(artworkCacheProvider).containsKey('fail'),
      isFalse,
    );

    shouldFail = false;
    await cache.ensureLoaded(['fail']);
    expect(env.fake.getTrackCalls.where((id) => id == 'fail').length, 2);
    expect(env.container.read(artworkCacheProvider)['fail'], isNotNull);
  });

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

  test(
    'concurrent ensureLoaded with different ids still loads claimed ids',
    () async {
      final gates = <String, Completer<LibraryTrackSummary?>>{};
      final env = boot(
        FakeLibraryTransport(
          getTrackHandler: (id) {
            final gate = gates.putIfAbsent(id, Completer.new);
            return gate.future;
          },
        ),
      );
      final cache = env.container.read(artworkCacheProvider.notifier);

      // A claims x,y then B replaces _wanted with y,z — x must still load.
      final first = cache.ensureLoaded(['x', 'y']);
      await pumpEventQueue();
      final second = cache.ensureLoaded(['y', 'z']);
      await pumpEventQueue();

      expect(env.fake.getTrackCalls.toSet(), {'x', 'y', 'z'});
      expect(
        env.fake.getTrackCalls.where((id) => id == 'y').length,
        1,
        reason: 'y must stay coalesced while in flight',
      );

      for (final id in ['x', 'y', 'z']) {
        gates[id]!.complete(
          LibraryTrackSummary(
            id: id,
            displayName: id,
            path: '/$id.flac',
            artwork: Uint8List.fromList([1]),
          ),
        );
      }
      await Future.wait([first, second]);

      final state = env.container.read(artworkCacheProvider);
      expect(state['x'], isNotNull);
      expect(state['y'], isNotNull);
      expect(state['z'], isNotNull);
    },
  );

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

  test('eviction falls back to wanted entries to keep the hard cap', () async {
    final env = boot(FakeLibraryTransport());
    final cache = env.container.read(artworkCacheProvider.notifier);

    await cache.ensureLoaded([for (var i = 0; i < 300; i++) 'id$i']);
    expect(env.container.read(artworkCacheProvider).length, 300);

    // Keep every filled id wanted so non-wanted victims are empty.
    await cache.ensureLoaded([for (var i = 0; i < 300; i++) 'id$i', 'id300']);
    final state = env.container.read(artworkCacheProvider);
    expect(state.containsKey('id300'), isTrue);
    expect(state.length, 300);
    // Insertion-order FIFO among wanted: oldest keys drop first.
    expect(state.containsKey('id0'), isFalse);
  });
}
