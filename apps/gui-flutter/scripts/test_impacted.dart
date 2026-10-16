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

/// Runs `git` in the project root and returns stdout lines.
List<String> _git(List<String> args) {
  final result = Process.runSync('git', args, workingDirectory: _projectRoot);
  if (result.exitCode != 0) return const [];
  return (result.stdout as String)
      .split('\n')
      .map((line) => line.trim())
      .where((line) => line.isNotEmpty)
      .toList();
}

/// Every path that differs from the base branch plus anything uncommitted.
Set<String> _changedFiles() {
  final base = _git(['merge-base', 'HEAD', 'origin/main']).isNotEmpty
      ? _git(['merge-base', 'HEAD', 'origin/main']).first
      : (_git(['merge-base', 'HEAD', 'main']).isNotEmpty
            ? _git(['merge-base', 'HEAD', 'main']).first
            : 'HEAD');

  final changed = <String>{
    ..._git(['diff', '--name-only', base]),
    ..._git(['diff', '--name-only']), // unstaged
    ..._git(['diff', '--cached', '--name-only']), // staged
    // Untracked files: a brand-new lib or test file is not in `git diff`.
    ..._git(['ls-files', '--others', '--exclude-standard']),
  };
  return changed;
}

/// Imports of [file], normalized to paths relative to the app root.
Set<String> _importsOf(String file) {
  final source = File(p.join(_projectRoot, file)).readAsStringSync();
  const packagePrefix = 'package:gui_flutter/';
  final out = <String>{};
  for (final match in RegExp(r"""import\s+'([^']+)'""").allMatches(source)) {
    final target = match.group(1)!;
    if (target.startsWith(packagePrefix)) {
      out.add(p.join('lib', target.substring(packagePrefix.length)));
    } else if (!target.startsWith('package:') && !target.startsWith('dart:')) {
      out.add(p.normalize(p.join(p.dirname(file), target)));
    }
  }
  return out;
}

/// Every `.dart` file under `lib/` and `test/`, mapped to its imports.
Map<String, Set<String>> _importGraph() {
  final graph = <String, Set<String>>{};
  for (final dir in const ['lib', 'test']) {
    final root = Directory(p.join(_projectRoot, dir));
    if (!root.existsSync()) continue;
    for (final entity in root.listSync(recursive: true)) {
      if (entity is! File || !entity.path.endsWith('.dart')) continue;
      final relative = p.relative(entity.path, from: _projectRoot);
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
  if (relative.startsWith('lib/l10n/') && relative.endsWith('.arb')) {
    return const ['lib/l10n/app_localizations.dart'];
  }
  if (_frbInputs.contains(relative) ||
      relative.startsWith('rust_builder/') ||
      relative.startsWith('crates/host-flutter/')) {
    return frbFiles.toList()..sort();
  }
  if (relative.endsWith('.dart') &&
      (relative.startsWith('lib/') || relative.startsWith('test/'))) {
    return [relative];
  }
  return const [];
}

Impact _impactOf(Set<String> changed, Map<String, Set<String>> graph) {
  final frbFiles = graph.keys
      .where((file) => file.startsWith('lib/src/rust/'))
      .toSet();

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

  final changed = _changedFiles();
  if (changed.isEmpty) {
    stdout.writeln('No changed files: nothing to run.');
    return;
  }

  final graph = _importGraph();
  final impact = _impactOf(changed, graph);
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
      final seeds = _seedsFor(
        file,
        graph.keys.where((f) => f.startsWith('lib/src/rust/')).toSet(),
      );
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

Future<int> _flutterTest(List<String> files) {
  return Process.start(
    'flutter',
    ['test', ...files],
    workingDirectory: _projectRoot,
    mode: ProcessStartMode.inheritStdio,
  ).then((process) => process.exitCode);
}
