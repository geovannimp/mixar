import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

/// Listens for MIDI mapping offers and prompts to enable (Tauri toast flow).
class ControllerOfferBridge extends ConsumerStatefulWidget {
  const ControllerOfferBridge({super.key});

  @override
  ConsumerState<ControllerOfferBridge> createState() =>
      _ControllerOfferBridgeState();
}

class _ControllerOfferBridgeState extends ConsumerState<ControllerOfferBridge> {
  ProviderSubscription<AsyncValue<ControllerTransport?>>? _listen;
  StreamSubscription<ControllerEvt>? _events;
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
    _events = transport.subscribeEvents().listen(_onEvent);
    _hydrate(transport);
  }

  void _hydrate(ControllerTransport transport) {
    // ponytail: 250ms × 40 poll of pendingOffers covers offers raised before
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

  void _onEvent(ControllerEvt evt) {
    if (!mounted) {
      return;
    }
    switch (evt.kind) {
      case ControllerEvtKind.mappingOffer:
        _showOffer(evt);
      case ControllerEvtKind.mappingAttached:
        ref.read(attachedMappingIdProvider.notifier).set(evt.mappingId);
        ref.invalidate(controllerDevicesProvider);
      case ControllerEvtKind.mappingDetached:
        if (ref.read(attachedMappingIdProvider) == evt.mappingId) {
          ref.read(attachedMappingIdProvider.notifier).set(null);
        }
        ref.invalidate(controllerDevicesProvider);
    }
  }

  void _showOffer(ControllerEvt evt) {
    final port = evt.portName;
    final mappingId = evt.mappingId;
    if (port == null || mappingId == null || _shownPorts.contains(port)) {
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
    showFToast(
      context: context,
      duration: null,
      onDismiss: () => _shownPorts.remove(port),
      title: Text('${evt.deviceName ?? mappingId} connected'),
      description: StatefulBuilder(
        builder: (context, setLocal) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text('Do you want to use this controller?'),
              const SizedBox(height: 8),
              FCheckbox(
                value: alwaysAllow,
                label: const Text('Always allow this device'),
                onChange: (v) => setLocal(() => alwaysAllow = v),
              ),
            ],
          );
        },
      ),
      suffixBuilder: (context, entry) => FButton(
        size: .sm,
        mainAxisSize: .min,
        onPress: () {
          entry.dismiss();
          unawaited(
            _enable(
              transport: transport,
              mappingId: mappingId,
              portName: port,
              deviceId: evt.deviceId,
              alwaysAllow: alwaysAllow,
            ),
          );
        },
        child: const Text('Enable'),
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
      showFToast(context: context, variant: .destructive, title: Text('$e'));
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
