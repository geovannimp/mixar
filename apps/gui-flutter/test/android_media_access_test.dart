import 'package:flutter/foundation.dart'
    show TargetPlatform, debugDefaultTargetPlatformOverride;
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/android_media_access.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final channel = MethodChannel(androidMediaAccessChannel);
  final calls = <MethodCall>[];

  /// Runs the next call as Android, with [handler] answering the channel. A
  /// missing [handler] leaves the channel unregistered, the shape an old build
  /// without `MainActivity`'s plugin has.
  void useAndroid([Future<Object?>? Function(MethodCall call)? handler]) {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    if (handler == null) {
      return;
    }
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) {
          calls.add(call);
          return handler(call);
        });
  }

  /// Stages [handler] as Android and asserts the helper reports no access.
  Future<void> expectNoAccess([
    Future<Object?>? Function(MethodCall call)? handler,
  ]) async {
    useAndroid(handler);
    expect(await ensureAudioReadAccess(), isFalse);
  }

  tearDown(() {
    debugDefaultTargetPlatformOverride = null;
    calls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test('non-Android platforms read paths directly, no prompt', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    expect(await ensureAudioReadAccess(), isTrue);
    expect(calls, isEmpty);
  });

  test('Android asks the host and reports a granted permission', () async {
    useAndroid((_) async => true);
    expect(await ensureAudioReadAccess(), isTrue);
    expect(calls.single.method, androidEnsureAudioReadMethod);
  });

  test('Android reports a refused permission as no access', () async {
    await expectNoAccess((_) async => false);
  });

  test('a channel failure counts as no access', () async {
    await expectNoAccess((_) async => throw PlatformException(code: 'busy'));
  });

  test('an unexpected channel exception counts as no access', () async {
    await expectNoAccess((_) async => throw Exception('host handler blew up'));
  });

  test('a missing host channel counts as no access', () async {
    await expectNoAccess();
  });
}
