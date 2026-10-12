import 'package:flutter/painting.dart';

/// Solid Mixar colour scales (Tailwind-aligned).
///
/// Prefer these over `withValues(alpha: …)` / `Color.alphaBlend` for fills,
/// borders, and icons. Alpha stays for true see-through surfaces only
/// (dialogs, dimming scrims) that must show content behind them.
///
/// Brand signal is teal. Deck identity is sky (A) / rose (B) — never a third
/// green/emerald ladder.
abstract final class MixarTeal {
  static const s50 = Color(0xFFF0FDFA);
  static const s100 = Color(0xFFCCFBF1);
  static const s200 = Color(0xFF99F6E4);
  static const s300 = Color(0xFF5EEAD4);
  static const s400 = Color(0xFF2DD4BF);
  static const s500 = Color(0xFF14B8A6);
  static const s600 = Color(0xFF0D9488);
  static const s700 = Color(0xFF0F766E);
  static const s800 = Color(0xFF115E59);
  static const s900 = Color(0xFF134E4A);
  static const s950 = Color(0xFF042F2E);
}

/// Sky scale — Deck A identity (website `--deck-a`, fader A grip).
abstract final class MixarSky {
  static const s50 = Color(0xFFF0F9FF);
  static const s100 = Color(0xFFE0F2FE);
  static const s200 = Color(0xFFBAE6FD);
  static const s300 = Color(0xFF7DD3FC);
  static const s400 = Color(0xFF38BDF8);
  static const s500 = Color(0xFF0EA5E9);
  static const s600 = Color(0xFF0284C7);
  static const s700 = Color(0xFF0369A1);
  static const s800 = Color(0xFF075985);
  static const s900 = Color(0xFF0C4A6E);
  static const s950 = Color(0xFF082F49);
}

/// Rose scale — Deck B identity (website `--deck-b`, fader B grip).
abstract final class MixarRose {
  static const s50 = Color(0xFFFFF1F2);
  static const s100 = Color(0xFFFFE4E6);
  static const s200 = Color(0xFFFECDD3);
  static const s300 = Color(0xFFFDA4AF);
  static const s400 = Color(0xFFFB7185);
  static const s500 = Color(0xFFF43F5E);
  static const s600 = Color(0xFFE11D48);
  static const s700 = Color(0xFFBE123C);
  static const s800 = Color(0xFF9F1239);
  static const s900 = Color(0xFF881337);
  static const s950 = Color(0xFF4C0519);
}

/// Amber scale — cue / caution chrome.
abstract final class MixarAmber {
  static const s50 = Color(0xFFFFFBEB);
  static const s100 = Color(0xFFFEF3C7);
  static const s200 = Color(0xFFFDE68A);
  static const s300 = Color(0xFFFCD34D);
  static const s400 = Color(0xFFFBBF24);
  static const s500 = Color(0xFFF59E0B);
  static const s600 = Color(0xFFD97706);
  static const s700 = Color(0xFFB45309);
  static const s800 = Color(0xFF92400E);
  static const s900 = Color(0xFF78350F);
  static const s950 = Color(0xFF451A03);
}
