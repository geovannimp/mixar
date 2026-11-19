import 'dart:io';

import 'package:build_tool/src/kache_shims.dart';
import 'package:test/test.dart';

void main() {
  late Directory temp;

  setUp(() {
    temp = Directory.systemTemp.createTempSync('kache-shims-test');
  });

  tearDown(() {
    temp.deleteSync(recursive: true);
  });

  group('resolveKacheShimsDir', () {
    test('returns the configured directory when it exists', () {
      final resolved = resolveKacheShimsDir(
        environment: {'KACHE_SHIMS_DIR': temp.path},
        isWindows: false,
      );

      expect(resolved, temp.path);
    });

    test('returns null when unset and HOME is unavailable', () {
      expect(
        resolveKacheShimsDir(environment: {}, isWindows: false),
        isNull,
      );
    });

    test('falls back to the default dir under HOME', () {
      final home = Directory('${temp.path}/home')..createSync(recursive: true);
      final shims = Directory('${home.path}/.local/lib/kache/shims')
        ..createSync(recursive: true);

      final resolved = resolveKacheShimsDir(
        environment: {'HOME': home.path},
        isWindows: false,
      );

      expect(resolved, shims.path);
    });

    test('prefers KACHE_SHIMS_DIR over HOME', () {
      final configured = Directory('${temp.path}/configured')..createSync();
      final home = Directory('${temp.path}/home')..createSync();
      Directory('${home.path}/.local/lib/kache/shims')
          .createSync(recursive: true);

      final resolved = resolveKacheShimsDir(
        environment: {
          'KACHE_SHIMS_DIR': configured.path,
          'HOME': home.path,
        },
        isWindows: false,
      );

      expect(resolved, configured.path);
    });

    test('returns null when empty', () {
      expect(
        resolveKacheShimsDir(
          environment: {'KACHE_SHIMS_DIR': ''},
          isWindows: false,
        ),
        isNull,
      );
    });

    test('returns null when the directory is missing', () {
      expect(
        resolveKacheShimsDir(
          environment: {'KACHE_SHIMS_DIR': '${temp.path}/missing'},
          isWindows: false,
        ),
        isNull,
      );
    });

    test('returns null on Windows', () {
      expect(
        resolveKacheShimsDir(
          environment: {'KACHE_SHIMS_DIR': temp.path},
          isWindows: true,
        ),
        isNull,
      );
    });
  });

  group('withKacheShims', () {
    test('leaves the environment untouched without a shims directory', () {
      final environment = {'CC': 'cc'};

      final result = withKacheShims(
        environment: environment,
        parentEnvironment: {'PATH': '/usr/bin'},
        shimsDir: null,
        isWindows: false,
      );

      expect(result, same(environment));
    });

    test('prepends the shims directory on Unix', () {
      final result = withKacheShims(
        environment: {'CC': 'cc'},
        parentEnvironment: {'PATH': '/usr/bin:/bin'},
        shimsDir: '/shims',
        isWindows: false,
      );

      expect(result['PATH'], '/shims:/usr/bin:/bin');
      expect(result['CC'], 'cc');
    });

    test('uses the Windows separator', () {
      final result = withKacheShims(
        environment: {},
        parentEnvironment: {'PATH': r'C:\Windows'},
        shimsDir: r'C:\shims',
        isWindows: true,
      );

      expect(result['PATH'], r'C:\shims;C:\Windows');
    });

    test('tolerates a missing parent PATH', () {
      final result = withKacheShims(
        environment: {},
        parentEnvironment: {},
        shimsDir: '/shims',
        isWindows: false,
      );

      expect(result['PATH'], '/shims:');
    });
  });
}
