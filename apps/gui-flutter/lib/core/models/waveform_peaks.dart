class SpectralPeak {
  const new({required this.low, required this.mid, required this.high});

  final double low;
  final double mid;
  final double high;
}

List<SpectralPeak> decodeRgbPeaks(List<int> bytes) {
  if (bytes.length < 3 || bytes.length % 3 != 0) {
    return const [];
  }
  final peaks = <SpectralPeak>[];
  for (var i = 0; i < bytes.length; i += 3) {
    peaks.add(
      SpectralPeak(
        low: bytes[i] / 255.0,
        mid: bytes[i + 1] / 255.0,
        high: bytes[i + 2] / 255.0,
      ),
    );
  }
  return peaks;
}
