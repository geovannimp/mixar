import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Highest 1-based channel the mapping asks for (2 when it declares none).
int controllerAudioMaxChannel(ControllerAudioHints hints) {
  var max = 2;
  for (final pair in [hints.masterChannels, hints.headphoneChannels]) {
    if (pair == null) {
      continue;
    }
    max = [max, pair.left, pair.right].reduce((a, b) => a > b ? a : b);
  }
  return max;
}

/// Finds the controller's own output among [devices].
///
/// Names match case-insensitively by substring. An exact name wins over a
/// partial one, then a device with enough channels for the declared routes,
/// then the one the system made default.
OutputDevice? matchControllerAudioOutput(
  List<OutputDevice> devices,
  ControllerAudioHints hints,
) {
  final needles = hints.outputNameContains
      .map((name) => name.trim().toLowerCase())
      .where((name) => name.isNotEmpty)
      .toList();
  if (needles.isEmpty) {
    return null;
  }
  final requiredChannels = controllerAudioMaxChannel(hints);
  OutputDevice? best;
  var bestScore = -1;
  for (final device in devices) {
    if (device.id == 'default' || device.maxChannels < requiredChannels) {
      continue;
    }
    final name = device.name.toLowerCase();
    var score = -1;
    for (final needle in needles) {
      if (name == needle) {
        score = 4;
        break;
      }
      if (name.contains(needle)) {
        score = 2;
      }
    }
    if (score < 0) {
      continue;
    }
    if (device.isDefault) {
      score += 1;
    }
    if (score > bestScore ||
        (score == bestScore &&
            best != null &&
            device.maxChannels > best.maxChannels)) {
      best = device;
      bestScore = score;
    }
  }
  return best;
}

/// Whether [route] already sends audio to [device]. The `default` alias counts
/// when the OS made that device the default output.
bool busRouteUsesDevice(BusRouteSettings route, OutputDevice device) =>
    busRouteUsesDeviceId(route, device.id) ||
    (route.deviceId == 'default' && device.isDefault);

/// Exact device-id match between a saved route and an override target.
bool busRouteUsesDeviceId(BusRouteSettings route, String deviceId) =>
    route.deviceId == deviceId;

/// Whether the controller's declared output is already in use, so the prompt
/// can stay quiet after the user has switched (or configured it by hand).
bool controllerAudioAlreadyRouted(
  AppSettings settings,
  ControllerAudioHints hints,
  OutputDevice device,
) {
  if (hints.masterChannels != null) {
    return busRouteUsesDevice(settings.masterBus, device);
  }
  if (hints.headphoneChannels != null) {
    return settings.previewEnabled &&
        busRouteUsesDevice(settings.previewBus, device);
  }
  return false;
}

/// Whether this controller is configured to switch the output buses on attach
/// without asking.
bool controllerAudioAlwaysSwitch(
  AppSettings settings,
  String controllerDeviceId,
) =>
    controllerDeviceId.isNotEmpty &&
    settings.alwaysSwitchAudioDeviceIds.contains(controllerDeviceId);

/// Whether [master] (or the preview bus) currently runs on the device a
/// controller mapping moved it to. Drives the Settings badges.
bool controllerAudioOverridesBus(AppSettings settings, {required bool master}) {
  final override = settings.controllerAudioOverride;
  if (override == null) {
    return false;
  }
  if (master) {
    return override.masterOverridden &&
        busRouteUsesDeviceId(settings.masterBus, override.outputDeviceId);
  }
  return override.previewOverridden &&
      settings.previewEnabled &&
      busRouteUsesDeviceId(settings.previewBus, override.outputDeviceId);
}

/// Applies the mapping's routes to [settings], or returns `null` when there is
/// nothing to change. A headphone route also turns the preview bus on: the
/// controller's headphone jack is useless otherwise.
///
/// The displaced routes are recorded in [AppSettings.controllerAudioOverride]
/// so detaching the controller can put them back.
AppSettings? applyControllerAudioOutputs(
  AppSettings settings,
  ControllerAudioHints hints,
  OutputDevice device, {
  required String mappingId,
  required String controllerLabel,
}) {
  final master = hints.masterChannels;
  final headphone = hints.headphoneChannels;
  if (master == null && headphone == null) {
    return null;
  }
  if (controllerAudioAlreadyRouted(settings, hints, device)) {
    return null;
  }
  final existing = settings.controllerAudioOverride;
  final sameOverride = existing != null && existing.mappingId == mappingId;
  final override = ControllerAudioOverride(
    mappingId: mappingId,
    controllerLabel: controllerLabel,
    outputDeviceId: device.id,
    // Keep the first displaced routes when the same mapping re-applies.
    previousMasterBus: sameOverride
        ? existing.previousMasterBus
        : settings.masterBus,
    masterOverridden: master != null,
    previousPreviewEnabled: sameOverride
        ? existing.previousPreviewEnabled
        : settings.previewEnabled,
    previousPreviewBus: sameOverride
        ? existing.previousPreviewBus
        : settings.previewBus,
    previewOverridden: headphone != null,
  );
  var updated = copyAppSettings(settings, controllerAudioOverride: override);
  if (master != null) {
    updated = copyAppSettings(
      updated,
      masterBus: _routeTo(updated.masterBus, device, master, hints.sampleRate),
    );
  }
  if (headphone != null) {
    updated = copyAppSettings(
      updated,
      previewEnabled: true,
      previewBus: _routeTo(
        updated.previewBus,
        device,
        headphone,
        hints.sampleRate,
      ),
    );
  }
  return updated;
}

/// Puts back the routes displaced by [mappingId]'s override.
///
/// A bus the user changed by hand while the controller was attached keeps its
/// new device; only buses still on the override's device are restored.
/// Returns `null` when this mapping has no active override.
({AppSettings settings, bool routesRestored})? restoreControllerAudioOutputs(
  AppSettings settings,
  String mappingId,
) {
  final override = settings.controllerAudioOverride;
  if (override == null || override.mappingId != mappingId) {
    return null;
  }
  var updated = settings;
  var routesRestored = false;
  if (override.masterOverridden &&
      busRouteUsesDeviceId(updated.masterBus, override.outputDeviceId)) {
    updated = copyAppSettings(updated, masterBus: override.previousMasterBus);
    routesRestored = true;
  }
  if (override.previewOverridden &&
      updated.previewEnabled &&
      busRouteUsesDeviceId(updated.previewBus, override.outputDeviceId)) {
    updated = copyAppSettings(
      updated,
      previewEnabled: override.previousPreviewEnabled,
      previewBus: override.previousPreviewBus,
    );
    routesRestored = true;
  }
  updated = copyAppSettings(updated, clearControllerAudioOverride: true);
  return (settings: updated, routesRestored: routesRestored);
}

BusRouteSettings _routeTo(
  BusRouteSettings current,
  OutputDevice device,
  ControllerChannelPair channels,
  int? sampleRate,
) => BusRouteSettings(
  deviceId: device.id,
  leftChannel: channels.left,
  rightChannel: channels.right,
  mode: BusChannelMode.stereo,
  sampleRate: sampleRate,
  lowLatency: current.lowLatency,
);
