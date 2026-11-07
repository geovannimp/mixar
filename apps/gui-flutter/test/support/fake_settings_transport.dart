import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// In-memory settings host for widget tests that boot the setup flow.
class FakeSettingsTransport implements SettingsTransport {
  new(this.stored, {this.saveError});

  AppSettings stored;

  /// When set, [saveSettings] throws it instead of storing.
  Object? saveError;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  Future<AppSettings> getSettings() async => stored;

  @override
  Stream<RealtimeStemsBenchmarkProgress> benchmarkRealtimeStems() =>
      const Stream<RealtimeStemsBenchmarkProgress>.empty();

  @override
  Future<RealtimeStemsCapability?> realtimeStemsCapability() async => null;

  @override
  Future<AppSettings> saveSettings({required AppSettings settings}) async {
    final error = saveError;
    if (error != null) {
      throw error;
    }
    stored = settings;
    return settings;
  }

  @override
  Future<AppSettings> setControllerAudioOverride({
    ControllerAudioOverride? overrideSettings,
  }) async {
    return stored = copyAppSettings(
      stored,
      controllerAudioOverride: overrideSettings,
      clearControllerAudioOverride: overrideSettings == null,
    );
  }

  @override
  Future<AppSettings> clearControllerAudioOverride({
    required String mappingId,
  }) async {
    if (stored.controllerAudioOverride?.mappingId == mappingId) {
      stored = copyAppSettings(stored, clearControllerAudioOverride: true);
    }
    return stored;
  }
}
