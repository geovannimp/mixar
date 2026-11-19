/// kache compiler shims for the Cargo build.
///
/// The shims cache C/C++ compiled by build scripts (Opus, SQLCipher,
/// OpenSSL) across worktrees. They are injected into the cargo child
/// environment only: on the ambient PATH Flutter's build hooks resolve
/// `clang` to the shim and then fail looking for the toolchain next to it.
library;

import 'dart:io';

/// Resolves kache's compiler-shim directory, or null when it is not usable.
///
/// [environment] is normally `Platform.environment`. `KACHE_SHIMS_DIR` is the
/// configured source (set in `mise.toml`, created by `mise run kache-setup`).
/// Xcode/Gradle/CMake can launch the build tool without mise's environment, so
/// fall back to kache's default directory under `HOME` instead of silently
/// disabling C/C++ caching. Windows has no `kache init --shims`.
String? resolveKacheShimsDir({
  required Map<String, String> environment,
  required bool isWindows,
}) {
  if (isWindows) {
    return null;
  }
  final configured = environment['KACHE_SHIMS_DIR'];
  final home = environment['HOME'];
  final dir = (configured != null && configured.isNotEmpty)
      ? configured
      : (home == null || home.isEmpty)
          ? null
          : '$home/.local/lib/kache/shims';
  if (dir == null || !Directory(dir).existsSync()) {
    return null;
  }
  return dir;
}

/// Prepends [shimsDir] to `PATH` in [environment], if [shimsDir] is not null.
///
/// [parentEnvironment] supplies the inherited `PATH`; other variables are kept
/// untouched. The separator is chosen explicitly so the Windows behavior is
/// testable off Windows.
Map<String, String> withKacheShims({
  required Map<String, String> environment,
  required Map<String, String> parentEnvironment,
  required String? shimsDir,
  required bool isWindows,
}) {
  if (shimsDir == null) {
    return environment;
  }
  final separator = isWindows ? ';' : ':';
  return {
    ...environment,
    'PATH': [shimsDir, parentEnvironment['PATH'] ?? ''].join(separator),
  };
}
