class SpectralPeak {
  const SpectralPeak({
    required this.low,
    required this.mid,
    required this.high,
  });

  final double low;
  final double mid;
  final double high;
}

class DetailWindow {
  const DetailWindow({
    required this.peaks,
    required this.startMs,
    required this.endMs,
  });

  final List<SpectralPeak> peaks;
  final int startMs;
  final int endMs;
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

SpectralPeak peakAtTime(
  List<SpectralPeak> overview,
  DetailWindow? detail,
  int durationMs,
  double timeMs, {
  bool fallbackToOverview = true,
}) {
  if (detail != null &&
      detail.peaks.length > 1 &&
      timeMs >= detail.startMs &&
      timeMs <= detail.endMs) {
    final span = detail.endMs - detail.startMs;
    if (span > 0) {
      final frac = (timeMs - detail.startMs) / span;
      return interpolatePeak(detail.peaks, frac * (detail.peaks.length - 1));
    }
  }
  if (!fallbackToOverview || overview.isEmpty || durationMs <= 0) {
    return const SpectralPeak(low: 0, mid: 0, high: 0);
  }
  final frac = (timeMs / durationMs).clamp(0.0, 1.0);
  return interpolatePeak(overview, frac * (overview.length - 1));
}

SpectralPeak interpolatePeak(List<SpectralPeak> peaks, double index) {
  if (peaks.isEmpty) {
    return const SpectralPeak(low: 0, mid: 0, high: 0);
  }
  final clamped = index.clamp(0.0, peaks.length - 1);
  final i0 = clamped.floor();
  final i1 = (i0 + 1).clamp(0, peaks.length - 1);
  final t = clamped - i0;
  final p0 = peaks[i0];
  final p1 = peaks[i1];
  return SpectralPeak(
    low: p0.low * (1 - t) + p1.low * t,
    mid: p0.mid * (1 - t) + p1.mid * t,
    high: p0.high * (1 - t) + p1.high * t,
  );
}
