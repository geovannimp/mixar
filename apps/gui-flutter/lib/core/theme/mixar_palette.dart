import 'package:flutter/painting.dart';

// Solid Mixar colour scales (Tailwind-aligned).
//
// Prefer these over `withValues(alpha: …)` / `Color.alphaBlend` for fills,
// borders, and icons. Alpha stays for true see-through surfaces only
// (dialogs, dimming scrims) that must show content behind them.
//
// Brand signal is teal. Deck identity is sky (A) / rose (B) — never a third
// green/emerald ladder.

/// Zinc scale — neutral chrome: hairlines, borders, muted text.
///
/// The dark surfaces already sit on this ladder (`background` is zinc-950,
/// `foreground` zinc-100, `mutedForeground` zinc-400), so a hairline can name a
/// solid step instead of compositing translucent white over whatever is behind.
abstract final class MixarZinc {
  static const s50 = Color(0xFFFAFAFA);
  static const s100 = Color(0xFFF4F4F5);
  static const s200 = Color(0xFFE4E4E7);
  static const s300 = Color(0xFFD4D4D8);
  static const s400 = Color(0xFFA1A1AA);
  static const s500 = Color(0xFF71717A);
  static const s600 = Color(0xFF52525B);
  static const s700 = Color(0xFF3F3F46);
  static const s800 = Color(0xFF27272A);
  static const s900 = Color(0xFF18181B);
  static const s950 = Color(0xFF09090B);
}

/// Bone scale — warm neutral chrome for the light theme.
///
/// Light chrome is paper, not a screen: the light surfaces sit on this ladder
/// (`background` bone-100, `card` bone-50, `mutedForeground` bone-600). Nothing
/// reaches pure white, and card sits 9 luminance points above the page so a
/// panel reads as a plane instead of a flat sheet. Mirrors the website light
/// tokens `--bg` / `--bg-elevated` / `--bg-card`; keep the two in sync.
abstract final class MixarBone {
  static const s50 = Color(0xFFFAF9F6);
  static const s100 = Color(0xFFEFEEE9);
  static const s200 = Color(0xFFE6E3DC);
  static const s300 = Color(0xFFD6D2C9);
  static const s400 = Color(0xFFB8B3A7);
  static const s500 = Color(0xFF8A857B);
  static const s600 = Color(0xFF57534E);
  static const s700 = Color(0xFF3D3A36);
  static const s800 = Color(0xFF2A2825);
  static const s900 = Color(0xFF1C1917);
}

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

/// Blue scale — accent palette option.
abstract final class MixarBlue {
  static const s50 = Color(0xFFEFF6FF);
  static const s100 = Color(0xFFDBEAFE);
  static const s200 = Color(0xFFBFDBFE);
  static const s300 = Color(0xFF93C5FD);
  static const s400 = Color(0xFF60A5FA);
  static const s500 = Color(0xFF3B82F6);
  static const s600 = Color(0xFF2563EB);
  static const s700 = Color(0xFF1D4ED8);
  static const s800 = Color(0xFF1E40AF);
  static const s900 = Color(0xFF1E3A8A);
  static const s950 = Color(0xFF172554);
}

/// Violet scale — accent palette option.
abstract final class MixarViolet {
  static const s50 = Color(0xFFF5F3FF);
  static const s100 = Color(0xFFEDE9FE);
  static const s200 = Color(0xFFDDD6FE);
  static const s300 = Color(0xFFC4B5FD);
  static const s400 = Color(0xFFA78BFA);
  static const s500 = Color(0xFF8B5CF6);
  static const s600 = Color(0xFF7C3AED);
  static const s700 = Color(0xFF6D28D9);
  static const s800 = Color(0xFF5B21B6);
  static const s900 = Color(0xFF4C1D95);
  static const s950 = Color(0xFF2E1065);
}

/// Orange scale — accent palette option.
abstract final class MixarOrange {
  static const s50 = Color(0xFFFFF7ED);
  static const s100 = Color(0xFFFFEDD5);
  static const s200 = Color(0xFFFED7AA);
  static const s300 = Color(0xFFFDBA74);
  static const s400 = Color(0xFFFB923C);
  static const s500 = Color(0xFFF97316);
  static const s600 = Color(0xFFEA580C);
  static const s700 = Color(0xFFC2410C);
  static const s800 = Color(0xFF9A3412);
  static const s900 = Color(0xFF7C2D12);
  static const s950 = Color(0xFF431407);
}

/// Pink scale — accent palette option.
///
/// Tailwind fuchsia, not pink: at the deep steps the picker uses for washes,
/// pink's hue drifts to maroon (pink-950 `#500724`) and the selected row reads
/// red. Fuchsia keeps every step on the magenta side of the hue wheel.
abstract final class MixarPink {
  static const s50 = Color(0xFFFDF4FF);
  static const s100 = Color(0xFFFAE8FF);
  static const s200 = Color(0xFFF5D0FE);
  static const s300 = Color(0xFFF0ABFC);
  static const s400 = Color(0xFFE879F9);
  static const s500 = Color(0xFFD946EF);
  static const s600 = Color(0xFFC026D3);
  static const s700 = Color(0xFFA21CAF);
  static const s800 = Color(0xFF86198F);
  static const s900 = Color(0xFF701A75);
  static const s950 = Color(0xFF4A044E);
}
