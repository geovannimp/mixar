import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/analytics/analytics_providers.dart';
import 'package:gui_flutter/analytics/analytics_transport.dart';
import 'package:gui_flutter/analytics/error_reporting.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/analytics.dart';
import 'package:gui_flutter/src/rust/api/settings.dart' show AppSettings;

class _FakeTransport implements AnalyticsTransport {
  final reported = <String>[];

  @override
  bool get isDisposed => false;

  @override
  void dispose() {}

  @override
  Future<void> appLaunched() async {}

  @override
  Future<void> setEnabled({required bool enabled}) async {}

  @override
  Future<void> reportError({
    required String source,
    required String kind,
    required String message,
    String? stack,
  }) async {
    reported.add(message);
  }
}

/// Snapshot both process-wide handlers and restore them after the test.
///
/// Call this before installing anything: `installErrorReporting` replaces both
/// handlers, and a leftover wrapper would chain into the next test.
void _restoreHandlers() {
  final previousFlutter = FlutterError.onError;
  final previousPlatform = PlatformDispatcher.instance.onError;
  addTearDown(() {
    FlutterError.onError = previousFlutter;
    PlatformDispatcher.instance.onError = previousPlatform;
  });
}

ProviderContainer _container({
  required AnalyticsTransport transport,
  required AppSettings settings,
}) {
  final container = ProviderContainer(
    overrides: [
      analyticsTransportProvider.overrideWith((ref) async => transport),
      appSettingsProvider.overrideWith((ref) async => settings),
    ],
  );
  addTearDown(container.dispose);
  return container;
}

/// Report the same error shape everywhere: the tests only vary who receives it.
void _reportFlutterError() => FlutterError.reportError(
  FlutterErrorDetails(exception: StateError('boom'), stack: StackTrace.current),
);

/// Install reporting with the Analytics switch at [enabled], report one error,
/// and hand back the transport so the test can assert what arrived.
Future<_FakeTransport> _reportWithSwitch({required bool enabled}) async {
  _restoreHandlers();
  final transport = _FakeTransport();
  final container = _container(
    transport: transport,
    settings: copyAppSettings(defaultAppSettings(), analyticsEnabled: enabled),
  );
  installAnalyticsErrorReporting(container);
  _reportFlutterError();
  await pumpEventQueue();
  return transport;
}

void main() {
  test('forwards framework errors and keeps the previous handler', () {
    _restoreHandlers();
    final reports = <ErrorReport>[];
    final previousReports = <FlutterErrorDetails>[];
    FlutterError.onError = previousReports.add;

    installErrorReporting(reports.add);
    _reportFlutterError();

    expect(previousReports, hasLength(1));
    final report = reports.single;
    expect(report.source, 'flutter');
    expect(report.kind, 'StateError');
    expect(report.message, 'Bad state: boom');
    expect(report.stack, isNotNull);
  });

  test('forwards uncaught asynchronous errors and stays unhandled', () {
    _restoreHandlers();
    final reports = <ErrorReport>[];
    final previousReports = <Object>[];
    PlatformDispatcher.instance.onError = (error, stack) {
      previousReports.add(error);
      return false;
    };

    installErrorReporting(reports.add);
    final handled = PlatformDispatcher.instance.onError!(
      ArgumentError('bad'),
      StackTrace.current,
    );

    expect(handled, isFalse);
    expect(previousReports.single, isA<ArgumentError>());
    final report = reports.single;
    expect(report.source, 'dart');
    expect(report.kind, 'ArgumentError');
    expect(report.message, 'Invalid argument(s): bad');
  });

  test('a throwing sink does not replace the original error', () {
    _restoreHandlers();
    installErrorReporting((_) => throw StateError('sink'));

    expect(_reportFlutterError, returnsNormally);
    final handled = PlatformDispatcher.instance.onError!(
      StateError('async boom'),
      StackTrace.current,
    );
    expect(handled, isFalse);
  });

  test('reports nothing while the Analytics switch is off', () async {
    expect((await _reportWithSwitch(enabled: false)).reported, isEmpty);
  });

  test('reports when the Analytics switch is on', () async {
    expect((await _reportWithSwitch(enabled: true)).reported, [
      'Bad state: boom',
    ]);
  });
}
