import 'package:flutter/painting.dart';

/// Solid Mixar colour scales (Tailwind-aligned).
///
/// Prefer these over `withValues(alpha: …)` / `Color.alphaBlend` for fills,
/// borders, and icons. Alpha stays for true see-through surfaces only
/// (dialogs, dimming scrims) that must show content behind them.
abstract final class MixarEmerald {
  static const s50 = Color(0xFFECFDF5);
  static const s100 = Color(0xFFD1FAE5);
  static const s200 = Color(0xFFA7F3D0);
  static const s300 = Color(0xFF6EE7B7);
  static const s400 = Color(0xFF34D399);
  static const s500 = Color(0xFF10B981);
  static const s600 = Color(0xFF059669);
  static const s700 = Color(0xFF047857);
  static const s800 = Color(0xFF065F46);
  static const s900 = Color(0xFF064E3B);
  static const s950 = Color(0xFF022C22);
}

/// Teal scale — brand accent / live transport chrome.
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
