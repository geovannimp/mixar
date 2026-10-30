import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_drop_hover.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Ink painted over artwork. Fixed instead of theme colours: the scrim is
/// always dark, so a light-theme `foreground` would vanish on it.
const _coverInk = Color(0xFFF4F4F5);

/// Scrim painted over the artwork while a track is dragged onto the deck.
const _dropScrimFill = Color(0xB309090B);

/// Unload chip fill: a small scrim at rest, a near-cover scrim when it grows
/// over the artwork (a touch of art still reads through), and a solid press.
const _chipFill = Color(0x99101014);
const _chipFillExpanded = Color(0xCC09090B);
const _chipFillActive = Color(0xE609090B);

/// Chip growth on hover/focus: size, corner radius and glyph scale together.
const _chipGrow = Duration(milliseconds: 110);

/// Deck cover — the deck's load surface.
///
/// The cover is what stands for the loaded track, so loading and unloading
/// live on it rather than in the header chip row with the deck settings:
///
/// * empty deck — a full-bleed ghost button (plus glyph, ghost wash on hover)
///   that opens the library, where a row is dragged or sent to the deck;
/// * loaded deck — the artwork carries an eject chip; hovering or focusing the
///   cover grows that chip until it fills the tile, so the whole cover is the
///   unload target, and a playing deck asks before it is cleared;
/// * a library or OS drag over this deck paints the drop glyph over the
///   artwork, so the drop lands where the user is looking.
///
/// The whole deck stays the drop target (`TrackDropZone`); this tile only
/// narrates it.
class DeckCover extends ConsumerStatefulWidget {
  const new({
    required this.deckId,
    required this.hasTrack,
    this.artwork,
    super.key,
  });

  final int deckId;
  final bool hasTrack;
  final Uint8List? artwork;

  @override
  ConsumerState<DeckCover> createState() => _DeckCoverState();
}

class _DeckCoverState extends ConsumerState<DeckCover> {
  String get _deckLabel => 'Deck ${String.fromCharCode(65 + widget.deckId)}';

  /// Unloads the deck. A playing deck asks first: clearing it is audible, and
  /// the mistake is unrecoverable — the track has to be loaded again.
  Future<void> _unload() async {
    // Read at click time: the deck can start or stop between builds.
    if (ref.read(deckPlayingProvider(widget.deckId))) {
      final confirmed = await showMixarConfirm<bool>(
        context: context,
        title: 'Unload $_deckLabel?',
        body: '$_deckLabel is playing. Unloading stops the track on this deck.',
        actions: const [
          MixarDialogAction(
            label: 'Cancel',
            value: false,
            variant: MixarButtonVariant.outline,
          ),
          MixarDialogAction(
            label: 'Unload',
            value: true,
            variant: MixarButtonVariant.destructive,
          ),
        ],
      );
      if (confirmed != true || !mounted) {
        return;
      }
      // A controller load can start while the dialog is up. The dialog is
      // already gone, so a silent return would read as a broken control.
      if (!ref.read(engineRunningProvider) ||
          ref.read(deckLoadingProvider(widget.deckId))) {
        showMixarToast(
          context: context,
          title: const Text('Deck is busy — unload cancelled'),
        );
        return;
      }
    }
    try {
      await unloadDeck(ref, widget.deckId);
    } on Object catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final loading = ref.watch(deckLoadingProvider(widget.deckId));
    final engineRunning = ref.watch(engineRunningProvider);
    final dropHover = DeckDropHover.of(context);
    // A load in flight owns the deck: replacing or ejecting under it would
    // race the host's prepare/apply.
    final ready = engineRunning && !loading;
    final loaded = widget.hasTrack;
    // Empty: the cover opens the library. Loaded: it is the unload target,
    // gated on the engine being free of a load.
    final canUnload = loaded && ready;

    return MTappable(
      onPress: !loaded
          ? () => openLibrary(ref)
          : canUnload
          ? () => unawaited(_unload())
          : null,
      semanticsLabel: loaded ? 'Unload $_deckLabel' : 'Load $_deckLabel',
      builder: (context, state) {
        final reveal = loaded && (state.hovered || state.focused);
        return Stack(
          fit: StackFit.expand,
          children: [
            Positioned.fill(
              child: loaded
                  ? _artwork(theme)
                  : AppTooltip(
                      tip: 'Load track',
                      // With the engine stopped the deck installs no drop
                      // target, so only promise the library.
                      description: engineRunning
                          ? 'Opens the library. Drag a track here, or drop one.'
                          : 'Opens the library.',
                      child: _emptySlot(theme, state, dropHover),
                    ),
            ),
            if (dropHover)
              Positioned.fill(child: IgnorePointer(child: _dropScrim())),
            if (loaded)
              Positioned.fill(
                child: LayoutBuilder(
                  builder: (context, constraints) {
                    // Rest: a 20 px chip inset in the corner. Hover/focus:
                    // the chip grows until it is the cover, so the whole tile
                    // is the unload target.
                    final extent = constraints.biggest.shortestSide;
                    return Align(
                      alignment: Alignment.topRight,
                      child: AnimatedPadding(
                        duration: _chipGrow,
                        curve: Curves.easeOutCubic,
                        padding: EdgeInsets.all(reveal ? 0 : 3),
                        child: _UnloadChip(
                          onPress: canUnload
                              ? () => unawaited(_unload())
                              : null,
                          expanded: reveal,
                          side: reveal ? extent : 20,
                          tip: 'Unload $_deckLabel',
                        ),
                      ),
                    );
                  },
                ),
              ),
            // Painted last so a grown chip cannot cover the focus ring; it is
            // an IgnorePointer, so it never takes the chip's press.
            if (state.focused)
              Positioned.fill(
                child: IgnorePointer(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: theme.colors.accent,
                        width: theme.style.borderWidth,
                      ),
                      borderRadius: _leftRadius(theme),
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _artwork(MixarThemeData theme) {
    final radius = _leftRadius(theme);
    final disc = Icon(
      LucideIcons.disc3,
      size: 20,
      color: theme.colors.mutedForeground,
    );
    final bytes = widget.artwork;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.background,
        borderRadius: radius,
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: bytes != null && bytes.isNotEmpty
            ? Image.memory(
                bytes,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => Center(child: disc),
              )
            : Center(child: disc),
      ),
    );
  }

  /// Ghost load button: no frame and no inset — the tile itself is the button,
  /// so its wash fills the whole 64 px square like any other ghost control.
  Widget _emptySlot(
    MixarThemeData theme,
    MTappableState state,
    bool dropHover,
  ) {
    final accented = state.active || dropHover;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: accented ? theme.colors.selection : null,
        borderRadius: _leftRadius(theme),
      ),
      child: Center(
        child: Icon(
          LucideIcons.plus,
          size: 20,
          color: accented
              ? theme.colors.accent
              : theme.colors.secondaryForeground,
        ),
      ),
    );
  }

  /// Only a drag paints over the artwork; a playing/hover cover is the eject
  /// button instead.
  Widget _dropScrim() {
    return const ColoredBox(
      color: _dropScrimFill,
      child: Center(
        child: Icon(LucideIcons.arrowDownToLine, size: 20, color: _coverInk),
      ),
    );
  }
}

/// Card-edge corners stay square on the right; the card clips the left. Flush
/// fills (artwork, ghost wash, grown chip, focus ring) all follow it.
BorderRadius _leftRadius(MixarThemeData theme) =>
    theme.style.borderRadius.md.copyWith(
      topRight: Radius.zero,
      bottomRight: Radius.zero,
      bottomLeft: Radius.zero,
    );

/// Eject chip that clears the deck. Deliberately not an [MTappable]: its own
/// hover region would beat the tile's, so the tile could not know to grow it.
/// It inherits the tile's hover, cursor and semantics (the tile is the control,
/// so there is no second tab stop) and keeps only its own press paint.
class _UnloadChip extends StatefulWidget {
  const new({
    required this.onPress,
    required this.expanded,
    required this.side,
    required this.tip,
  });

  final VoidCallback? onPress;

  /// Hover tip, deck-qualified so four decks stay distinguishable.
  final String tip;

  /// Deck tile hovered/focused — the chip grows to [side] and covers the art.
  final bool expanded;

  final double side;

  @override
  State<_UnloadChip> createState() => _UnloadChipState();
}

class _UnloadChipState extends State<_UnloadChip> {
  var _pressed = false;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final enabled = widget.onPress != null;
    final fill = _pressed
        ? _chipFillActive
        : widget.expanded
        ? _chipFillExpanded
        : _chipFill;
    final body = GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: enabled ? (_) => setState(() => _pressed = true) : null,
      onTapUp: enabled ? (_) => setState(() => _pressed = false) : null,
      onTapCancel: enabled ? () => setState(() => _pressed = false) : null,
      onTap: widget.onPress,
      child: AnimatedContainer(
        duration: _chipGrow,
        curve: Curves.easeOutCubic,
        width: widget.side,
        height: widget.side,
        decoration: BoxDecoration(
          color: fill,
          borderRadius: widget.expanded
              ? _leftRadius(theme)
              : theme.style.borderRadius.sm,
        ),
        child: Center(
          child: AnimatedScale(
            duration: _chipGrow,
            curve: Curves.easeOutCubic,
            scale: widget.expanded ? 2 : 1,
            child: Icon(
              LucideIcons.eject,
              size: 11,
              color: enabled ? _coverInk : _coverInk.withValues(alpha: 0.55),
            ),
          ),
        ),
      ),
    );
    return AppTooltip(
      tip: widget.tip,
      description: 'Clears the track from this deck.',
      child: body,
    );
  }
}
