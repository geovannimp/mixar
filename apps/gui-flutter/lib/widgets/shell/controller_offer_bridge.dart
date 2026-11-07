import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/controller_audio.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/bus_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/bus.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_checkbox.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Listens for MIDI mapping offers and prompts to enable (Tauri toast flow).
class ControllerOfferBridge extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<ControllerOfferBridge> createState() =>
      _ControllerOfferBridgeState();
}

class _ControllerOfferBridgeState extends ConsumerState<ControllerOfferBridge> {
  ProviderSubscription<AsyncValue<ControllerTransport?>>? _listen;
  ProviderSubscription<AsyncValue<BusTransport>>? _busListen;
  StreamSubscription<Event>? _events;
  Timer? _retry;
  final _shownPorts = <String>{};

  /// Ports already asked about switching audio, and those being resolved.
  final _audioAskedPorts = <String>{};
  final _audioPendingPorts = <String>{};

  @override
  void initState() {
    super.initState();
    _listen = ref.listenManual(controllerTransportProvider, (prev, next) {
      next.whenData(_bind);
    }, fireImmediately: true);
    // The bus resolves independently of the controller: bind again when it
    // arrives, otherwise the first offers would have no stream to arrive on.
    _busListen = ref.listenManual(busTransportProvider, (prev, next) {
      if (next.hasValue) {
        _bind(ref.read(controllerTransportProvider).asData?.value);
      }
    }, fireImmediately: true);
  }

  void _bind(ControllerTransport? transport) {
    _events?.cancel();
    _retry?.cancel();
    if (transport == null) {
      return;
    }
    // Mapping events ride the one bus stream, like every other host event.
    _events = listenBusEvents(ref, _onEvent, (e, _) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    });
    _hydrate(transport);
    unawaited(_hydrateAttachedAudio(transport));
  }

  void _hydrate(ControllerTransport transport) {
    // shortcut: 250ms × 40 poll of pendingOffers covers offers raised before
    // this widget subscribed (cold ALSA). Upgrade: Rust sink replays pending
    // offers on subscribe, then delete this timer.
    var attempts = 0;
    var reported = false;
    Future<void> tick() async {
      if (!mounted) {
        return;
      }
      try {
        final offers = await transport.pendingOffers();
        for (final event in offers) {
          _onEvent(event);
        }
        if (offers.isNotEmpty) {
          _retry?.cancel();
        }
      } catch (e, st) {
        if (reported) {
          return;
        }
        reported = true;
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: e,
            stack: st,
            context: ErrorDescription('controller pendingOffers hydrate'),
          ),
        );
      }
    }

    unawaited(tick());
    _retry = Timer.periodic(const Duration(milliseconds: 250), (_) {
      attempts += 1;
      unawaited(tick());
      if (attempts >= 40) {
        _retry?.cancel();
      }
    });
  }

  void _onEvent(Event event) {
    if (!mounted) {
      return;
    }
    switch (event) {
      case Event_MappingOffer(:final field0):
        _showOffer(field0);
      case Event_MappingAttached(:final field0):
        unawaited(syncAttachedMappingIds(ref));
        ref.invalidate(controllerDevicesProvider);
        unawaited(_maybeOfferAudioSwitch(field0.mappingId, field0.portName));
      case Event_MappingDetached(:final field0):
        _shownPorts.remove(field0.portName);
        _audioAskedPorts.remove(field0.portName);
        _audioPendingPorts.remove(field0.portName);
        dismissMixarToast(context, id: _offerToastId(field0.portName));
        dismissMixarToast(context, id: _audioToastId(field0.portName));
        unawaited(_restoreAudioOutputs(field0.mappingId));
        unawaited(syncAttachedMappingIds(ref));
        ref.invalidate(controllerDevicesProvider);
      default:
        break;
    }
  }

  static String _offerToastId(String port) => 'controller-offer:$port';

  static String _audioToastId(String port) => 'controller-audio:$port';

  /// Covers mappings that were already attached when this widget bound (a
  /// trusted controller plugged in before the shell subscribed). Also restores
  /// the saved buses when a controller that owned them is gone.
  Future<void> _hydrateAttachedAudio(ControllerTransport transport) async {
    try {
      final rows = await transport.listMappings();
      final attached = <String, ControllerAudioHints>{};
      for (final row in rows) {
        final audio = row.audio;
        if (row.attached && audio != null) {
          attached[row.id] = audio;
        }
      }
      final devices = await transport.listDevices();
      for (final device in devices) {
        final mappingId = device.matchedMappingId;
        if (device.direction == ControllerDeviceDirection.input &&
            mappingId != null &&
            attached.containsKey(mappingId)) {
          unawaited(_maybeOfferAudioSwitch(mappingId, device.portName));
        }
      }

      final settingsTransport = await ref.read(
        settingsTransportProvider.future,
      );
      final override =
          (await settingsTransport.getSettings()).controllerAudioOverride;
      if (override == null || !mounted) {
        return;
      }
      if (rows.any((row) => row.id == override.mappingId && row.attached)) {
        return;
      }
      var midiHints = const <String>[];
      for (final row in rows) {
        if (row.id == override.mappingId) {
          midiHints = row.midiNameContains;
          break;
        }
      }
      final portPresent = devices.any(
        (device) =>
            device.direction == ControllerDeviceDirection.input &&
            _portMatches(device.portName, midiHints),
      );
      if (!portPresent) {
        unawaited(_restoreIfControllerGone(override.mappingId, midiHints));
      }
    } catch (e, st) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: e,
          stack: st,
          context: ErrorDescription('controller attached audio hydrate'),
        ),
      );
    }
  }

  static bool _portMatches(String portName, List<String> hints) {
    final name = portName.toLowerCase();
    return hints.any((hint) {
      final needle = hint.trim().toLowerCase();
      return needle.isNotEmpty && name.contains(needle);
    });
  }

  /// Startup recovery: a controller unplugged while the app was closed leaves
  /// an override pointing at a device that is no longer present. Wait one poll
  /// so a trusted controller can auto-attach before restoring.
  Future<void> _restoreIfControllerGone(
    String mappingId,
    List<String> midiHints,
  ) async {
    await Future<void>.delayed(const Duration(milliseconds: 900));
    if (!mounted) {
      return;
    }
    try {
      final transport = ref.read(controllerTransportProvider).value;
      if (transport == null) {
        return;
      }
      for (final row in await transport.listMappings()) {
        if (row.id == mappingId && row.attached) {
          return;
        }
      }
      final devices = await transport.listDevices();
      final portPresent = devices.any(
        (device) =>
            device.direction == ControllerDeviceDirection.input &&
            _portMatches(device.portName, midiHints),
      );
      if (!portPresent) {
        await _restoreAudioOutputs(mappingId);
      }
    } catch (e, st) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: e,
          stack: st,
          context: ErrorDescription('controller audio output recovery'),
        ),
      );
    }
  }

  /// Asks once per port whether to move Mixar's output buses onto the
  /// controller's own audio interface.
  Future<void> _maybeOfferAudioSwitch(String mappingId, String portName) async {
    if (!mounted ||
        _audioAskedPorts.contains(portName) ||
        _audioPendingPorts.contains(portName)) {
      return;
    }
    _audioPendingPorts.add(portName);
    try {
      final transport = ref.read(controllerTransportProvider).value;
      if (transport == null) {
        return;
      }
      ControllerMappingInfo? mapping;
      for (final row in await transport.listMappings()) {
        if (row.id == mappingId) {
          mapping = row;
          break;
        }
      }
      final hints = mapping?.audio;
      if (hints == null ||
          (hints.masterChannels == null && hints.headphoneChannels == null)) {
        return;
      }
      // Hot-plug: the audio device can appear a beat after the MIDI port.
      OutputDevice? device;
      for (var attempt = 0; attempt < 4; attempt++) {
        if (attempt > 0) {
          await Future<void>.delayed(const Duration(milliseconds: 350));
        }
        if (!mounted) {
          return;
        }
        ref.invalidate(audioDevicesProvider);
        final devices = await ref.read(audioDevicesProvider.future);
        device = matchControllerAudioOutput(devices, hints);
        if (device != null) {
          break;
        }
      }
      if (device == null) {
        return;
      }
      final settingsTransport = await ref.read(
        settingsTransportProvider.future,
      );
      final current = await settingsTransport.getSettings();
      if (controllerAudioAlreadyRouted(current, hints, device)) {
        return;
      }
      if (!mounted) {
        return;
      }
      final productName = mapping?.productName.trim() ?? '';
      final controllerDeviceId = mapping?.deviceId ?? '';
      final label = productName.isEmpty ? device.name : productName;
      _audioAskedPorts.add(portName);
      if (controllerAudioAlwaysSwitch(current, controllerDeviceId)) {
        await _switchAudioOutputs(
          portName: portName,
          mappingId: mappingId,
          controllerDeviceId: controllerDeviceId,
          label: label,
          hints: hints,
          device: device,
        );
        return;
      }
      _showAudioOffer(
        portName: portName,
        mappingId: mappingId,
        controllerDeviceId: controllerDeviceId,
        label: label,
        hints: hints,
        device: device,
      );
    } catch (e, st) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: e,
          stack: st,
          context: ErrorDescription('controller audio output offer'),
        ),
      );
    } finally {
      _audioPendingPorts.remove(portName);
    }
  }

  void _showAudioOffer({
    required String portName,
    required String mappingId,
    required String controllerDeviceId,
    required String label,
    required ControllerAudioHints hints,
    required OutputDevice device,
  }) {
    final master = hints.masterChannels;
    final headphones = hints.headphoneChannels;
    var alwaysSwitch = false;
    showMixarToast(
      context: context,
      id: _audioToastId(portName),
      duration: null,
      title: Text('Switch audio outputs to $label?'),
      description: StatefulBuilder(
        builder: (context, setLocal) => Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (master != null)
              Text('Master → $label (channels ${master.left}–${master.right})'),
            if (headphones != null)
              Text(
                'Headphones → $label '
                '(channels ${headphones.left}–${headphones.right})',
              ),
            const SizedBox(height: 4),
            const Text('Mixar will restart the audio engine.'),
            const SizedBox(height: 8),
            MixarCheckbox(
              value: alwaysSwitch,
              label: const Text('Always switch audio to this controller'),
              onChanged: (value) => setLocal(() => alwaysSwitch = value),
            ),
          ],
        ),
      ),
      suffixBuilder: (context, dismiss) => Row(
        mainAxisSize: MainAxisSize.min,
        spacing: 8,
        children: [
          AppButton(
            size: .sm,
            mainAxisSize: .min,
            variant: .outline,
            onPress: dismiss,
            child: const Text('Keep current'),
          ),
          AppButton(
            size: .sm,
            mainAxisSize: .min,
            onPress: () {
              dismiss();
              unawaited(
                _switchAudioOutputs(
                  portName: portName,
                  mappingId: mappingId,
                  controllerDeviceId: controllerDeviceId,
                  label: label,
                  hints: hints,
                  device: device,
                  alwaysSwitch: alwaysSwitch,
                ),
              );
            },
            child: const Text('Switch outputs'),
          ),
        ],
      ),
    );
  }

  Future<void> _switchAudioOutputs({
    required String portName,
    required String mappingId,
    required String controllerDeviceId,
    required String label,
    required ControllerAudioHints hints,
    required OutputDevice device,
    bool alwaysSwitch = false,
  }) async {
    try {
      final settingsTransport = await ref.read(
        settingsTransportProvider.future,
      );
      final current = await settingsTransport.getSettings();
      var updated = applyControllerAudioOutputs(
        current,
        hints,
        device,
        mappingId: mappingId,
        controllerLabel: label,
      );
      if (updated == null) {
        return;
      }
      if (alwaysSwitch &&
          controllerDeviceId.isNotEmpty &&
          !updated.alwaysSwitchAudioDeviceIds.contains(controllerDeviceId)) {
        updated = copyAppSettings(
          updated,
          alwaysSwitchAudioDeviceIds: [
            ...updated.alwaysSwitchAudioDeviceIds,
            controllerDeviceId,
          ],
        );
      }
      final result = await saveAppSettings(ref, updated);
      if (!mounted) {
        return;
      }
      if (result.applyError != null) {
        _audioAskedPorts.remove(portName);
        showMixarToast(
          context: context,
          variant: MixarToastVariant.destructive,
          title: const Text('Could not switch audio outputs'),
          description: Text(result.applyError!),
        );
        return;
      }
      showMixarToast(
        context: context,
        title: Text('Audio outputs switched to $label'),
      );
    } catch (e, st) {
      _audioAskedPorts.remove(portName);
      if (!mounted) {
        return;
      }
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: e,
          stack: st,
          context: ErrorDescription('controller audio output switch'),
        ),
      );
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: const Text('Could not switch audio outputs'),
        description: Text('$e'),
      );
    }
  }

  /// Puts the pre-controller buses back when its mapping detaches.
  Future<void> _restoreAudioOutputs(String mappingId) async {
    try {
      final settingsTransport = await ref.read(
        settingsTransportProvider.future,
      );
      final current = await settingsTransport.getSettings();
      final restored = restoreControllerAudioOutputs(current, mappingId);
      if (restored == null || !mounted) {
        return;
      }
      if (restored.routesRestored) {
        await saveAppSettings(ref, restored.settings);
      } else {
        // The user moved the buses by hand; only drop the stale marker so no
        // engine restart is needed for a route that did not change.
        await settingsTransport.saveSettings(settings: restored.settings);
        ref.invalidate(appSettingsProvider);
      }
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        title: const Text('Audio outputs restored'),
      );
    } catch (e, st) {
      if (!mounted) {
        return;
      }
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: e,
          stack: st,
          context: ErrorDescription('controller audio output restore'),
        ),
      );
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: const Text('Could not restore audio outputs'),
        description: Text('$e'),
      );
    }
  }

  void _showOffer(MappingOffer offer) {
    final port = offer.portName;
    final mappingId = offer.mappingId;
    if (_shownPorts.contains(port)) {
      return;
    }
    if (!mounted) {
      return;
    }
    final transport = ref.read(controllerTransportProvider).value;
    if (transport == null) {
      return;
    }
    _shownPorts.add(port);
    var alwaysAllow = false;
    // Default true: any dismiss path that does not explicitly opt out (Cancel)
    // clears [port] from [_shownPorts] so a later re-plug / re-offer can show.
    // Cancel sets this false before dismiss so hydrate does not re-prompt.
    // Enable keeps the default (true). Detach also clears via [_onEvent].
    var clearPortOnDismiss = true;
    showMixarToast(
      context: context,
      id: _offerToastId(port),
      duration: null,
      onDismiss: () {
        if (clearPortOnDismiss) {
          _shownPorts.remove(port);
        }
      },
      title: Text('${offer.deviceName} connected'),
      description: StatefulBuilder(
        builder: (context, setLocal) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text('Do you want to use this controller?'),
              const SizedBox(height: 8),
              MixarCheckbox(
                value: alwaysAllow,
                label: const Text('Always allow this device'),
                onChanged: (v) => setLocal(() => alwaysAllow = v),
              ),
            ],
          );
        },
      ),
      suffixBuilder: (context, dismiss) => Row(
        mainAxisSize: MainAxisSize.min,
        spacing: 8,
        children: [
          AppButton(
            size: .sm,
            mainAxisSize: .min,
            variant: .outline,
            onPress: () {
              clearPortOnDismiss = false;
              dismiss();
              unawaited(_restoreAudioOutputs(mappingId));
            },
            child: const Text('Cancel'),
          ),
          AppButton(
            size: .sm,
            mainAxisSize: .min,
            onPress: () {
              clearPortOnDismiss = true;
              dismiss();
              unawaited(
                _enable(
                  transport: transport,
                  mappingId: mappingId,
                  portName: port,
                  deviceId: offer.deviceId,
                  alwaysAllow: alwaysAllow,
                ),
              );
            },
            child: const Text('Enable'),
          ),
        ],
      ),
    );
  }

  Future<void> _enable({
    required ControllerTransport transport,
    required String mappingId,
    required String portName,
    required String? deviceId,
    required bool alwaysAllow,
  }) async {
    try {
      if (alwaysAllow && deviceId != null && deviceId.isNotEmpty) {
        final settings = await ref.read(settingsTransportProvider.future);
        final current = await settings.getSettings();
        final trusted = List<String>.from(current.trustedControllerDeviceIds);
        if (!trusted.contains(deviceId)) {
          trusted.add(deviceId);
        }
        await settings.saveSettings(
          settings: copyAppSettings(
            current,
            trustedControllerDeviceIds: trusted,
          ),
        );
        ref.invalidate(appSettingsProvider);
        ref.invalidate(controllerTransportProvider);
        ref.invalidate(controllerMappingsProvider);
        ref.invalidate(controllerDevicesProvider);
        return;
      }
      await transport.enableMapping(mappingId: mappingId, portName: portName);
    } catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    }
  }

  @override
  void dispose() {
    _listen?.close();
    _busListen?.close();
    _events?.cancel();
    _retry?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
