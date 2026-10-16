import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/artwork_cache.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Artwork thumbnail for a list row (tracks + history).
class TrackArtworkThumb extends ConsumerWidget {
  const new({required this.trackId, required this.size});

  final String trackId;
  final double size;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    Widget placeholder() => ColoredBox(
      color: theme.colors.muted,
      child: Center(
        child: Icon(
          LucideIcons.disc2,
          size: size * 0.57,
          color: theme.colors.mutedForeground,
        ),
      ),
    );
    final bytes = ref.watch(artworkCacheProvider.select((c) => c[trackId]));
    // One square box for both states. The placeholder needs the explicit
    // `SizedBox.square` too: left unconstrained it shrank to the icon's width
    // while the row stretched it to the full row height, so the thumb rendered
    // as a tall sliver instead of a square.
    return SizedBox.square(
      dimension: size,
      child: bytes != null && bytes.isNotEmpty
          ? Image.memory(
              bytes,
              width: size,
              height: size,
              fit: BoxFit.cover,
              cacheWidth: (size * 2).round(),
              cacheHeight: (size * 2).round(),
              errorBuilder: (_, _, _) => placeholder(),
            )
          : placeholder(),
    );
  }
}
