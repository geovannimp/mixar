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
