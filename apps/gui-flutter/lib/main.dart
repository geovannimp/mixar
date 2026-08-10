import 'package:flutter/material.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const SmokeApp());
}

class SmokeApp extends StatelessWidget {
  const SmokeApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'rust-mixer Flutter smoke',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.teal),
        useMaterial3: true,
      ),
      home: const SmokeHomePage(),
    );
  }
}

class SmokeHomePage extends StatefulWidget {
  const SmokeHomePage({super.key});

  @override
  State<SmokeHomePage> createState() => _SmokeHomePageState();
}

class _SmokeHomePageState extends State<SmokeHomePage> {
  List<String> _backends = const [];
  String? _backend;
  List<OutputDevice> _devices = const [];
  bool _running = false;
  String? _error;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _bootstrap();
  }

  Future<void> _bootstrap() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final backends = listBackendNames();
      final preferred = backends.contains('auto')
          ? 'auto'
          : (backends.isNotEmpty ? backends.first : null);
      setState(() {
        _backends = backends;
        _backend = preferred;
        _running = engineIsRunning();
      });
      if (preferred != null) {
        await _refreshDevices();
      }
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  Future<void> _refreshDevices() async {
    final backend = _backend;
    if (backend == null) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final devices = await listOutputDevices(backend: backend);
      setState(() {
        _devices = devices;
        _running = engineIsRunning();
      });
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  Future<void> _start() async {
    final backend = _backend;
    if (backend == null) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await startEngine(backend: backend);
      setState(() => _running = engineIsRunning());
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  Future<void> _stop() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await stopEngine();
      setState(() => _running = engineIsRunning());
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Engine smoke')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text('Status: ${_running ? "running" : "stopped"}'),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              // ignore: deprecated_member_use
              value: _backend,
              decoration: const InputDecoration(
                labelText: 'Backend',
                border: OutlineInputBorder(),
              ),
              items: _backends
                  .map((b) => DropdownMenuItem(value: b, child: Text(b)))
                  .toList(),
              onChanged: _busy
                  ? null
                  : (value) async {
                      setState(() => _backend = value);
                      await _refreshDevices();
                    },
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FilledButton(
                  onPressed: _busy ? null : _refreshDevices,
                  child: const Text('Refresh devices'),
                ),
                FilledButton(
                  onPressed: _busy || _running ? null : _start,
                  child: const Text('Start engine'),
                ),
                OutlinedButton(
                  onPressed: _busy || !_running ? null : _stop,
                  child: const Text('Stop engine'),
                ),
              ],
            ),
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            const SizedBox(height: 16),
            Text(
              'Output devices (${_devices.length})',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Expanded(
              child: _devices.isEmpty
                  ? const Text('No devices listed.')
                  : ListView.separated(
                      itemCount: _devices.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final d = _devices[index];
                        return ListTile(
                          title: Text(d.name),
                          subtitle: Text(
                            '${d.id} · ${d.maxChannels} ch'
                            '${d.isDefault ? " · default" : ""}',
                          ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
