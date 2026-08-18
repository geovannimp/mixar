/// Deck time as `m:ss.t` (Tauri `formatDeckTimeTenth`).
String formatDeckTimeTenth(int? ms) {
  if (ms == null || ms < 0) {
    return '—';
  }
  final totalTenths = ms ~/ 100;
  final minutes = totalTenths ~/ 600;
  final rem = totalTenths % 600;
  final whole = rem ~/ 10;
  final tenth = rem % 10;
  return '$minutes:${whole.toString().padLeft(2, '0')}.$tenth';
}

/// Beat length label: `1/32`, `1/2`, `1`, `4`.
String formatBeatLength(num beats) {
  if (beats >= 1) {
    return beats == beats.roundToDouble() ? '${beats.round()}' : '$beats';
  }
  if (beats <= 0) {
    return '$beats';
  }
  final den = (1 / beats).round();
  return '1/$den';
}
