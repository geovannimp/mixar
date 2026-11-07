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
bool controllerAudioAlreadyRouted(AppSettings settings, OutputDevice device) {
  final active = settings.controllerAudioOverride;
  return active != null && active.outputDeviceId == device.id;
}

/// Whether this controller is configured to switch the output buses on attach
/// without asking.
bool controllerAudioAlwaysSwitch(
  AppSettings settings,
  String controllerDeviceId,
) =>
    controllerDeviceId.isNotEmpty &&
    settings.alwaysSwitchAudioDeviceIds.contains(controllerDeviceId);

/// The master route the engine is actually running: the live override when one
/// owns master, otherwise the saved route.
BusRouteSettings effectiveMasterBus(AppSettings settings) {
  final override = settings.controllerAudioOverride;
  return override != null && override.masterOverridden
      ? override.masterBus
      : settings.masterBus;
}

/// The preview route the engine is actually running.
BusRouteSettings effectivePreviewBus(AppSettings settings) {
  final override = settings.controllerAudioOverride;
  return override != null && override.previewOverridden
      ? override.previewBus
      : settings.previewBus;
}

/// Whether [master] (or the preview bus) currently runs on the device a
/// controller mapping moved it to. Drives the Settings badges.
bool controllerAudioOverridesBus(AppSettings settings, {required bool master}) {
  final override = settings.controllerAudioOverride;
  if (override == null) {
    return false;
  }
  return master ? override.masterOverridden : override.previewOverridden;
}

/// Builds the live bus override for a controller mapping.
///
/// The saved master/preview routes are only used for `lowLatency` and as the
/// inactive side of a single-bus mapping. Returning `null` means the mapping
/// declares no routes or they are already active.
ControllerAudioOverride? buildControllerAudioOverride(
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
  if (controllerAudioAlreadyRouted(settings, device)) {
    return null;
  }
  final existing = settings.controllerAudioOverride;
  final sameDevice =
      existing != null &&
      existing.mappingId == mappingId &&
      existing.outputDeviceId == device.id;
  return ControllerAudioOverride(
    mappingId: mappingId,
    controllerLabel: controllerLabel,
    outputDeviceId: device.id,
    masterOverridden: master != null,
    masterBus: master == null
        ? (sameDevice ? existing.masterBus : settings.masterBus)
        : _routeTo(settings.masterBus, device, master, hints.sampleRate),
    previewOverridden: headphone != null,
    previewBus: headphone == null
        ? (sameDevice ? existing.previewBus : settings.previewBus)
        : _routeTo(settings.previewBus, device, headphone, hints.sampleRate),
  );
}

/// Removes one bus from a live override, or returns `null` when neither bus is
/// overridden anymore.
ControllerAudioOverride? clearControllerAudioOverrideBus(
  ControllerAudioOverride override, {
  required bool master,
}) {
  final masterOverridden = !master && override.masterOverridden;
  final previewOverridden = master && override.previewOverridden;
  if (!masterOverridden && !previewOverridden) {
    return null;
  }
  return ControllerAudioOverride(
    mappingId: override.mappingId,
    controllerLabel: override.controllerLabel,
    outputDeviceId: override.outputDeviceId,
    masterOverridden: masterOverridden,
    masterBus: override.masterBus,
    previewOverridden: previewOverridden,
    previewBus: override.previewBus,
  );
}

/// Applies a manual bus edit, dropping the controller override for that bus so
/// saving the draft returns it to the saved route.
AppSettings editControllerAudioBus(
  AppSettings settings,
  BusRouteSettings route, {
  required bool master,
}) {
  final override = settings.controllerAudioOverride;
  final nextOverride = override == null
      ? null
      : clearControllerAudioOverrideBus(override, master: master);
  return copyAppSettings(
    settings,
    masterBus: master ? route : settings.masterBus,
    previewBus: master ? settings.previewBus : route,
    controllerAudioOverride: nextOverride,
    clearControllerAudioOverride: nextOverride == null,
  );
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
