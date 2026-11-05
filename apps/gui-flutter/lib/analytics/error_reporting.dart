import 'package:flutter/foundation.dart';

/// One error caught by the process-wide handlers, reduced to what error
/// tracking needs.
@immutable
class ErrorReport {
  const new({
    required this.source,
    required this.kind,
    required this.message,
    this.stack,
  });

  /// Where it was caught: `flutter` for framework errors, `dart` for uncaught
  /// asynchronous ones.
  final String source;

  /// The error's runtime type, which PostHog groups issues by.
  final String kind;

  final String message;
  final String? stack;
}

typedef ErrorReportSink = void Function(ErrorReport report);

/// Route framework and uncaught-asynchronous errors to [sink].
///
/// Additive: handlers installed earlier still run, so console output and any
/// crash presentation are unchanged. Nothing here decides consent — the sink
/// does, and the Rust transport drops reports while analytics is off.
void installErrorReporting(ErrorReportSink sink) {
  void forward(ErrorReport report) {
    try {
      sink(report);
    } on Object {
      // Reporting must never replace the error with one of its own.
    }
  }

  final previousFlutterError = FlutterError.onError;
  FlutterError.onError = (details) {
    (previousFlutterError ?? FlutterError.presentError)(details);
    forward(
      ErrorReport(
        source: 'flutter',
        kind: details.exception.runtimeType.toString(),
        message: details.exceptionAsString(),
        stack: details.stack?.toString(),
      ),
    );
  };

  final previousPlatformError = PlatformDispatcher.instance.onError;
  PlatformDispatcher.instance.onError = (error, stack) {
    forward(
      ErrorReport(
        source: 'dart',
        kind: error.runtimeType.toString(),
        message: error.toString(),
        stack: stack.toString(),
      ),
    );
    return previousPlatformError?.call(error, stack) ?? false;
  };
}
