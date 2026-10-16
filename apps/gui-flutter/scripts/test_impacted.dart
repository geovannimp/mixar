// Runs only the Flutter test suites impacted by the changed files, instead of
// the whole suite.
//
// How it decides (nothing else is consulted — no historical flake data, no
// name heuristics):
//
//   1. `pubspec.yaml` / `pubspec.lock` changed  -> the whole suite (dependency
//      resolution can change anything, including generated code).
//   2. `lib/l10n/*.arb` changed                 -> `lib/l10n/app_localizations.dart`
//      (every locale funnels through it, so one entry covers all generated
//      localizations).
//   3. `flutter_rust_bridge.yaml`, `rust_builder/**` or `crates/host-flutter/**`
//      changed                                 -> every `lib/src/rust/**` file, i.e.
//      the regenerated FRB surface.
//   4. Any other `.dart` under `lib/` or `test/` -> itself.
//   5. Anything else under `apps/gui-flutter/**` (platform glue, assets, docs,
//      moon.yml, analysis_options, …) and everything outside the app (other
//      crates, website, CI config) -> no Flutter-test impact at all: the Dart
//      suite never loads the Rust dylib and never loads assets.
//
// "Impact" is transitive: a suite is impacted when a changed file is reachable
// from it through the import graph (lib + test/support + test/helpers), which is
// what makes rule 2 and the support/ hops work.
//
// Usage:
//   mise exec -- dart run scripts/test_impacted.dart            # list the suites
//   mise exec -- dart run scripts/test_impacted.dart --run      # run them
//   mise exec -- dart run scripts/test_impacted.dart --verbose  # explain matches
//
// Exits 0 when there is nothing to run.

import 'dart:io';

import 'package:path/path.dart' as p;

/// Dependency manifests: these are the only inputs that force the full suite.
const _dependencyManifests = {'pubspec.yaml', 'pubspec.lock'};

/// Codegen inputs whose output lands in `lib/src/rust/**`.
const _frbInputs = <String>{'flutter_rust_bridge.yaml'};

const _appPrefix = 'apps/gui-flutter/';

/// Result of classifying the changed files: `seeds` are the files whose
/// reachability from a suite marks it impacted; `full` is true when only
/// running the whole suite is correct.
typedef Impact = ({Set<String> seeds, bool full});

String _projectRoot = '.';

/// git and `package:` URIs are always forward-slash, so graph keys match them
/// and both sides compare on any host platform.
String _toPosix(String path) => path.replaceAll(r'\', '/');

/// Runs `git` in the project root and returns stdout lines.
///
/// A nonzero exit throws: returning an empty list instead would make a failed
/// `git diff` indistinguishable from "nothing changed", and the tool would exit
/// 0 having skipped every suite.
List<String> _git(List<String> args) {
  final result = Process.runSync('git', args, workingDirectory: _projectRoot);
  if (result.exitCode != 0) {
    stderr
      ..writeln('git ${args.join(' ')} failed in $_projectRoot:')
      ..writeln((result.stderr as String).trim());
    exitCode = 1;
    throw ProcessException('git', args, 'git failed', result.exitCode);
  }
  return (result.stdout as String)
      .split('\n')
      .map((line) => line.trim())
      .where((line) => line.isNotEmpty)
      .toList();
}

/// Like [_git], but returns null instead of throwing — for probes where a
/// failure is an expected answer rather than an error.
List<String>? _tryGit(List<String> args) {
  final result = Process.runSync('git', args, workingDirectory: _projectRoot);
  if (result.exitCode != 0) return null;
  return (result.stdout as String)
      .split('\n')
      .map((line) => line.trim())
      .where((line) => line.isNotEmpty)
      .toList();
}

/// Candidate default-branch refs. The repo triggers CI on both `main` and
/// `develop`, so a branch cut from `develop` must find its base too.
const _defaultBranchRefs = <String>[
  'origin/HEAD',
  'origin/main',
  'main',
  'origin/develop',
  'develop',
];

/// The merge base with the default branch, or `null` when it cannot be
/// determined (shallow clone, detached HEAD, no remote refs).
String? _mergeBase() {
  for (final ref in _defaultBranchRefs) {
    final bases = _tryGit(['merge-base', 'HEAD', ref]);
    if (bases != null && bases.isNotEmpty) return bases.first;
  }
  return null;
}

/// Every path that differs from the base branch plus anything uncommitted.
///
/// Calls [onUnknownBase] when the merge base cannot be determined; the
/// branch diff is then invisible (only working-tree changes are visible), so
/// the caller must fall back to the full suite rather than under-select.
Set<String> _changedFiles({required void Function() onUnknownBase}) {
  final base = _mergeBase();
  if (base == null) onUnknownBase();

  final changed = <String>{
    if (base != null) ..._git(['diff', '--name-only', base]),
    ..._git(['diff', '--name-only']), // unstaged
    ..._git(['diff', '--cached', '--name-only']), // staged
    // Untracked files: a brand-new lib or test file is not in `git diff`.
    ..._git(['ls-files', '--others', '--exclude-standard']),
  };
  return changed;
}

/// Removes comments so a doc comment quoting `import 'x.dart';` cannot fake a
/// directive (they exist, e.g. in `lib/l10n/app_localizations.dart`).
///
/// Combined with [_directives], only statements whose first token is a
/// directive keyword are parsed, and only quoted URIs inside them count — so a
/// URL inside an ordinary string literal never becomes a graph edge. String
/// literals themselves are not stripped: doing so would remove the URIs.
String _stripComments(String source) {
  final withoutBlocks = source.replaceAll(
    RegExp(r'''/\*.*?\*/''', dotAll: true),
    ' ',
  );
  return withoutBlocks.replaceAll(RegExp(r'''//[^\n]*'''), '');
}

/// Targets of every `import` / `export` / `part` directive in [file].
///
/// All quoted URIs in a directive are kept, so a conditional import
/// (stub first, `if (dart.library.io) io.dart` second) contributes both
/// branches. On the VM the conditional branch is the compiled one, and keeping
/// the unused branch only over-selects, which is the safe direction.
///
/// Paths are posix-normalized because they are compared against git's
/// forward-slash output; `path` would emit backslashes on Windows and then
/// nothing would ever match.
Set<String> _importsOf(String file) {
  final source = _stripComments(
    File(p.join(_projectRoot, file)).readAsStringSync(),
  );
  const packagePrefix = 'package:gui_flutter/';
  final out = <String>{};
  for (final directive in _directives(source)) {
    final uris = RegExp('''['"]([^'"]+)['"]''');
    for (final uri in uris.allMatches(directive)) {
      final target = uri.group(1)!;
      if (target.startsWith(packagePrefix)) {
        out.add(p.posix.join('lib', target.substring(packagePrefix.length)));
      } else if (!target.startsWith('package:') &&
          !target.startsWith('dart:')) {
        out.add(p.posix.normalize(p.posix.join(p.posix.dirname(file), target)));
      }
    }
  }
  return out;
}

/// `;`-terminated statements of [source], then only `import` / `export` /
/// `part`.
///
/// Splitting on a bare `;` (or the `[^;]*` regex this replaces) is wrong in
/// both directions: a `;` inside a string literal cuts a directive short and
/// drops a real edge, silently under-selecting suites. Tracking quote state
/// removes that class of error.
Iterable<String> _directives(String source) {
  final statements = <String>[];
  final buffer = StringBuffer();
  var quote = '';
  for (var i = 0; i < source.length; i++) {
    final ch = source[i];
    if (ch == r'\') {
      buffer.write(ch);
      if (i + 1 < source.length) buffer.write(source[++i]);
      continue;
    }
    if (quote.isNotEmpty) {
      buffer.write(ch);
      if (ch == quote) quote = '';
      continue;
    }
    if (ch == "'" || ch == '"') {
      quote = ch;
      buffer.write(ch);
      continue;
    }
    if (ch == ';') {
      statements.add(buffer.toString());
      buffer.clear();
      continue;
    }
    buffer.write(ch);
  }
  if (buffer.isNotEmpty) statements.add(buffer.toString());
  return statements.where(
    (statement) =>
        RegExp(r'''^\s*(?:import|export|part)\b''').hasMatch(statement),
  );
}

/// Every `.dart` file under `lib/` and `test/`, mapped to its imports.
Map<String, Set<String>> _importGraph() {
  final graph = <String, Set<String>>{};
  for (final dir in const ['lib', 'test']) {
    final root = Directory(p.join(_projectRoot, dir));
    if (!root.existsSync()) continue;
    for (final entity in root.listSync(recursive: true)) {
      if (entity is! File || !entity.path.endsWith('.dart')) continue;
      final relative = _toPosix(p.relative(entity.path, from: _projectRoot));
      graph[relative] = _importsOf(relative);
    }
  }
  return graph;
}

/// All files reachable from [start] through the import graph.
Set<String> _reachable(String start, Map<String, Set<String>> graph) {
  final seen = <String>{};
  final stack = <String>[start];
  while (stack.isNotEmpty) {
    final current = stack.removeLast();
    if (!seen.add(current)) continue;
    stack.addAll(graph[current] ?? const <String>{});
  }
  return seen;
}

/// `apps/gui-flutter/lib/x.dart` -> `lib/x.dart`; out-of-app paths (e.g.
/// `crates/host-flutter/…`) are returned unchanged so the codegen rule still
/// matches them.
String _appRelative(String changed) {
  return changed.startsWith(_appPrefix)
      ? changed.substring(_appPrefix.length)
      : changed;
}

/// Translates one changed path into the graph seeds it should match.
List<String> _seedsFor(String changed, Set<String> frbFiles) {
  final relative = _appRelative(changed);
  if (_dependencyManifests.contains(relative)) return const [];
  if (relative == 'l10n.yaml' ||
      (relative.startsWith('lib/l10n/') && relative.endsWith('.arb'))) {
    return const ['lib/l10n/app_localizations.dart'];
  }
  if (_frbInputs.contains(relative) ||
      relative.startsWith('rust_builder/') ||
      relative.startsWith('crates/host-flutter/')) {
    return frbFiles.toList();
  }
  if (relative.endsWith('.dart') &&
      (relative.startsWith('lib/') || relative.startsWith('test/'))) {
    return [relative];
  }
  return const [];
}

Impact _impactOf(Set<String> changed, Set<String> frbFiles) {
  if (changed.any(
    (file) => _dependencyManifests.contains(_appRelative(file)),
  )) {
    return (seeds: <String>{}, full: true);
  }

  final seeds = <String>{};
  for (final file in changed) {
    seeds.addAll(_seedsFor(file, frbFiles));
  }
  return (seeds: seeds, full: false);
}

List<String> _suites(Map<String, Set<String>> graph) =>
    graph.keys
        .where(
          (file) => file.startsWith('test/') && file.endsWith('_test.dart'),
        )
        .toList()
      ..sort();

Future<void> main(List<String> args) async {
  final run = args.contains('--run');
  final verbose = args.contains('--verbose');

  final scriptDir = p.dirname(p.fromUri(Platform.script));
  _projectRoot = p.normalize(p.join(scriptDir, '..'));
  final projectName = p.basename(_projectRoot);

  var baseUnknown = false;
  final Set<String> changed;
  try {
    changed = _changedFiles(onUnknownBase: () => baseUnknown = true);
  } on ProcessException {
    // _git already printed the command and git's stderr. Rethrowing would
    // dump a stack trace and exit 255 instead of the intended 1.
    exitCode = 1;
    return;
  }
  if (baseUnknown) {
    // The branch diff is invisible here, so a targeted run would silently skip
    // whatever the branch changed. Must precede the `changed.isEmpty` check:
    // in a shallow clone (fetch-depth 1) with everything committed, the working
    // tree is empty and this would exit 0 having run zero suites.
    stderr.writeln(
      'warning: no merge base with the default branch — running the full '
      'suite instead of a possibly incomplete selection.',
    );
    stdout.writeln('Running the full suite ($projectName).');
    if (!run) {
      stdout.writeln('FULL');
      return;
    }
    exitCode = await _flutterTest(const []);
    return;
  }

  if (changed.isEmpty) {
    stdout.writeln('No changed files: nothing to run.');
    return;
  }

  final graph = _importGraph();
  final frbFiles = graph.keys
      .where((file) => file.startsWith('lib/src/rust/'))
      .toSet();

  final impact = _impactOf(changed, frbFiles);
  final suites = _suites(graph);

  if (impact.full) {
    stdout.writeln(
      'Dependency manifest changed -> running the full suite ($projectName).',
    );
    if (!run) {
      stdout.writeln('FULL');
      return;
    }
    exitCode = await _flutterTest(const []);
    return;
  }

  final impacted = suites
      .where((suite) => _reachable(suite, graph).any(impact.seeds.contains))
      .toList();

  if (verbose) {
    for (final file in changed.toList()..sort()) {
      final seeds = _seedsFor(file, frbFiles);
      stdout.writeln(
        '  ${_appRelative(file).padRight(52)} -> '
        '${seeds.isEmpty ? 'no test impact' : seeds.take(2).join(', ')}'
        '${seeds.length > 2 ? ' (+${seeds.length - 2})' : ''}',
      );
    }
  }

  if (impacted.isEmpty) {
    stdout.writeln(
      'No suite imports the changed files -> nothing to run '
      '(${changed.length} changed file(s)).',
    );
    return;
  }

  stdout.writeln(
    '${impacted.length} of ${suites.length} suites impacted by '
    '${changed.length} changed file(s).',
  );
  impacted.forEach(stdout.writeln);
  if (!run) return;
  exitCode = await _flutterTest(impacted);
}

/// Usable CPU count as a string.
///
/// `nproc` first, then the POSIX `getconf`, mirroring `scripts/flutter_test.sh`:
/// both honour the affinity mask (4 under `taskset -c 0-3`), which
/// `Platform.numberOfProcessors` does not — it reports every online CPU, so a
/// mask would over-subscribe `flutter test -j`.
String _cpuCount() {
  for (final command in const [
    ['nproc'],
    ['getconf', '_NPROCESSORS_ONLN'],
  ]) {
    final result = Process.runSync(command.first, command.sublist(1));
    final count = int.tryParse((result.stdout as String).trim());
    if (result.exitCode == 0 && count != null && count > 0) {
      return count.toString();
    }
  }
  return Platform.numberOfProcessors.toString();
}

/// Runs `flutter test [files]` with the same concurrency the `test` task uses.
/// Without `-j`, package:test falls back to cores/2 and loses that speedup
/// (4 cores: 48.3s -> 34.7s).
Future<int> _flutterTest(List<String> files) async {
  final Process process;
  try {
    process = await Process.start(
      'flutter',
      ['test', '-j', _cpuCount(), ...files],
      workingDirectory: _projectRoot,
      // Same GIT_* guard as scripts/flutter_test.sh and the format/lint/icon
      // tasks: Flutter's version probe otherwise inherits the hook's repo and
      // reports 0.0.0-unknown. `includeParentEnvironment: false` is required —
      // without it Dart merges the parent environment back in and the removals
      // are silently undone.
      includeParentEnvironment: false,
      environment: Map<String, String>.of(Platform.environment)
        ..remove('GIT_DIR')
        ..remove('GIT_WORK_TREE')
        ..remove('GIT_INDEX_FILE')
        ..remove('GIT_PREFIX'),
      mode: ProcessStartMode.inheritStdio,
    );
  } on ProcessException catch (error) {
    stderr
      ..writeln('error: could not start flutter: $error')
      ..writeln(
        'Run it through the toolchain: '
        'mise exec -- dart run scripts/test_impacted.dart --run',
      );
    return 1;
  }
  return await process.exitCode;
}
