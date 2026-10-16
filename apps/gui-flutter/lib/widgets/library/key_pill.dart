import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill_glyph.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Key pill: the same chip as the rest of the metadata, with the key label
/// taking the configured key colour. Watches the harmonic reference so a deck
/// key change repaints the row without rebuilding the list.
class KeyPill extends ConsumerWidget {
  const new({
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
    super.key,
  });

  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final reference = ref.watch(harmonicReferenceKeyProvider);
    final color = colorForKey(
      rawKey,
      keyColorMode,
      harmonicReferenceKey: reference,
    );
    return MetaPill(
      text: formatDeckKey(rawKey, keyDisplayMode),
      fillWidth: false,
      leading: const MetaPillGlyph(LucideIcons.music2),
      textColor: color ?? theme.colors.mutedForeground,
      fontWeight: color != null ? FontWeight.w600 : FontWeight.w500,
    );
  }
}
