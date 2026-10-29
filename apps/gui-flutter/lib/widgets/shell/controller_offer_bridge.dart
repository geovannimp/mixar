import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/providers/shell/bus_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
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
  StreamSubscription<Event>? _events;
  Timer? _retry;
  final _shownPorts = <String>{};

  @override
  void initState() {
    super.initState();
    _listen = ref.listenManual(controllerTransportProvider, (prev, next) {
      next.whenData(_bind);
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
      case Event_MappingAttached():
        unawaited(syncAttachedMappingIds(ref));
        ref.invalidate(controllerDevicesProvider);
      case Event_MappingDetached(:final field0):
        _shownPorts.remove(field0.portName);
        dismissMixarToast(context, id: _offerToastId(field0.portName));
        unawaited(syncAttachedMappingIds(ref));
        ref.invalidate(controllerDevicesProvider);
      default:
        break;
    }
  }

  static String _offerToastId(String port) => 'controller-offer:$port';

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
    _events?.cancel();
    _retry?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
