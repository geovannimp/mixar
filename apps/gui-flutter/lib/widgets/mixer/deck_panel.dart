import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/deck_cue_hold_session.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show EngineTransport;
import 'package:gui_flutter/widgets/mixer/deck_cue_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_track_info.dart';
import 'package:gui_flutter/widgets/mixer/deck_transport_shape.dart';
import 'package:gui_flutter/widgets/mixer/track_drop_zone.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Height of the transport buttons when they sit in their own row.
const _kTransportButtonHeight = 36.0;

/// Breathing room above and below the full-width transport buttons.
const _kTransportRowGap = 8.0;

/// Height the full-width transport block takes: the buttons plus the gap above
/// and below them. The divider hairline is added at layout time.
const _kStackedTransportHeight =
    _kTransportButtonHeight + _kTransportRowGap * 2;

/// Performance-panel height worth keeping before the transport row gives up
/// its space and moves beside the pads.
const _kMinStackedPanelHeight = 150.0;

/// Diameter cap for the compact transport circles beside the pads.
const _kCompactTransportDiameter = 72.0;

/// Fewest pixels the performance panel is worth beside the compact transport;
/// under it the pads and their tab rail cannot lay out, so the transport row
/// moves back under the panel.
const _kMinCompactPanelWidth = 120.0;

/// Smallest circle worth pressing; below it the transport takes the bottom
/// row instead of shrinking the pads' side column any further.
const _kMinCompactButtonDiameter = 48.0;

/// Side gutter for the deck's transport row and its divider, plus the pad
/// floor's breathing room.
const _kDeckSideInset = 10.0;

/// Deck chrome (track info, performance tabs, transport) + tempo column.
class DeckPanel extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.label,
    required this.accent,
    super.key,
  });

  final int deckId;
  final String label;
  final DeckAccent accent;

  bool get _tempoOnRight => deckId.isEven;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loadedTitle = ref.watch(deckTrackTitleProvider(deckId));
    final hasTrack = ref.watch(deckHasTrackProvider(deckId));
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final engineRunning = ref.watch(engineRunningProvider);
    final loading = ref.watch(deckLoadingProvider(deckId));
    final settings = ref.watch(appSettingsProvider).value;
    final loadDisabled = loading || !engineRunning;
    final transportDisabled = loadDisabled || !hasTrack;
    final isMaster = ref.watch(deckIsMasterProvider(deckId));
    final tempo = DeckTempoPanel(
      accent: accent,
      speed: ref.watch(deckSpeedProvider(deckId)),
      tempoRange: ref.watch(deckTempoRangeProvider(deckId)),
      syncMode: ref.watch(deckSyncModeProvider(deckId)),
      isMaster: isMaster,
      trackBpm: ref.watch(deckBpmProvider(deckId)),
      playbackRatio: ref.watch(deckPlaybackRatioProvider(deckId)),
      loading: skeleton,
      enabled: engineRunning,
      tempoRangeSteps: settings?.tempoRangeSteps ?? kTempoRangeSteps,
      onSpeedChange: (speed) {
        unawaited(_engineCmd(context, () => setDeckSpeed(ref, deckId, speed)));
      },
      onTempoRangeChange: (tempoRange) {
        unawaited(
          _engineCmd(context, () => setDeckTempoRange(ref, deckId, tempoRange)),
        );
      },
      onToggleSync: (beatSync) {
        unawaited(
          _engineCmd(
            context,
            () => toggleDeckSync(ref, deckId, beatSync: beatSync),
          ),
        );
      },
    );

    // Play/pause must not rebuild DeckTrackInfo / pads (artwork blink).
    final play = _DeckPlayButton(deckId: deckId, disabled: transportDisabled);
    final cue = _DeckCueControl(deckId: deckId, disabled: transportDisabled);
    // Cue/Play keep the deck's existing left-to-right order from the row.
    final transport = _tempoOnRight ? [cue, play] : [play, cue];

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DeckTrackInfo(deckId: deckId, hasTrack: hasTrack, title: loadedTitle),
        Expanded(
          child: LayoutBuilder(
            builder: (context, constraints) {
              final theme = context.theme;
              final borderWidth = theme.style.borderWidth;
              // One divider shape for both layouts so the two cannot drift.
              Widget transportDivider({Axis axis = Axis.horizontal}) =>
                  MDivider(
                    key: const ValueKey('deck-performance-transport-divider'),
                    axis: axis,
                    color: theme.colors.border,
                    thickness: borderWidth,
                    padding: EdgeInsets.zero,
                  );
              final performance = DeckPerformancePanel(
                deckId: deckId,
                hasTrack: hasTrack,
                accent: accent,
                // Engine-level only: pad mode must stay selectable while the
                // deck is empty, and the per-surface panels already gate
                // their controls on `hasTrack`.
                disabled: loadDisabled,
              );
              final available = constraints.maxHeight;
              // Tiny heights: the pair's own gap and the fixed button height
              // yield rather than overflow. The shell keeps decks far above
              // this; the guard keeps a degenerate pane from throwing.
              final pairGap = available >= 8 ? 8.0 : 0.0;
              final widthBudget =
                  (constraints.maxWidth -
                          _kMinCompactPanelWidth -
                          _kDeckSideInset * 3)
                      .clamp(0.0, double.infinity);
              // Fit the circles to the width the pads can spare before
              // giving the controls a bottom row: a smaller circle keeps the
              // panel's floor and the pads' full height.
              final diameter = ((available - pairGap) / 2)
                  .clamp(0.0, _kCompactTransportDiameter)
                  .clamp(0.0, widthBudget);
              final compactFits = diameter >= _kMinCompactButtonDiameter;
              // A short deck cannot spare a full-width transport row without
              // squeezing the pads to an unusable strip; a narrow one cannot
              // spare the side column. In both cases the controls take the
              // bottom row and the pads keep the room.
              // The breakpoint budgets the whole stacked block, divider
              // included, so the panel keeps its floor.
              final stackedBlock = _kStackedTransportHeight + borderWidth;
              if (available >= _kMinStackedPanelHeight + stackedBlock ||
                  !compactFits) {
                final rowGap = available >= stackedBlock
                    ? _kTransportRowGap
                    : 0.0;
                // The row never takes more than the deck's height: the pads
                // get the non-negative remainder above it.
                final buttonHeight = _kTransportButtonHeight.clamp(
                  0.0,
                  (available - rowGap * 2 - borderWidth).clamp(
                    0.0,
                    double.infinity,
                  ),
                );
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(child: performance),
                    // The divider hugs the panel; the row keeps equal
                    // breathing room above and below so the controls sit
                    // centred between it and the deck's foot.
                    if (rowGap > 0) transportDivider(),
                    Padding(
                      padding: EdgeInsets.symmetric(
                        horizontal: _kDeckSideInset,
                        vertical: rowGap,
                      ),
                      child: SizedBox(
                        height: buttonHeight,
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          spacing: 8,
                          children: [
                            for (final control in transport)
                              Expanded(child: control),
                          ],
                        ),
                      ),
                    ),
                  ],
                );
              }
              // Compact: the two controls are perfect circles, capped by the
              // column width and by the room a pair plus its gap needs. The
              // pair is centred in the pads area, leaving the rest to the pads.
              // The divider hugs the panel and the circles sit centred in the
              // gutter beside it; right-hand decks mirror the whole side.
              final transportOnLeft = !_tempoOnRight;
              final side = SizedBox(
                width: diameter + _kDeckSideInset * 2,
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  spacing: pairGap,
                  children: [
                    for (final control in transport)
                      SizedBox.square(dimension: diameter, child: control),
                  ],
                ),
              );
              final divider = transportDivider(axis: Axis.vertical);
              return Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: transportOnLeft
                    ? [side, divider, Expanded(child: performance)]
                    : [Expanded(child: performance), divider, side],
              );
            },
          ),
        ),
      ],
    );

    return Semantics(
      container: true,
      label: label,
      child: TrackDropZone(
        deckId: deckId,
        child: ClipRect(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (!_tempoOnRight) tempo,
              Expanded(child: body),
              if (_tempoOnRight) tempo,
            ],
          ),
        ),
      ),
    );
  }
}

/// Owns one ordered cue-hold session so begin/end never race.
class _DeckCueControl extends ConsumerStatefulWidget {
  const new({required this.deckId, required this.disabled});

  final int deckId;
  final bool disabled;

  @override
  ConsumerState<_DeckCueControl> createState() => _DeckCueControlState();
}

class _DeckCueControlState extends ConsumerState<_DeckCueControl> {
  final _session = DeckCueHoldSession();
  // Cached via listenManual — Riverpod forbids ref.read in dispose().
  EngineTransport? _engine;
  ProviderSubscription<AsyncValue<EngineTransport?>>? _engineListen;

  // Bound when a hold is requested, so the release always reaches the same
  // transport even if the provider publishes a replacement mid-hold.
  EngineTransport? _holdEngine;
  int? _holdDeckId;
  // Set when a release throws; a dispose that only awaited an already-queued
  // (swallowed) end uses it to retry that release once.
  var _lastReleaseFailed = false;

  @override
  void initState() {
    super.initState();
    _engineListen = ref.listenManual(engineTransportProvider, (prev, next) {
      _engine = next.asData?.value;
    }, fireImmediately: true);
  }

  void _beginHold() {
    final engine = _engine;
    final deckId = widget.deckId;
    if (engine == null || _holdEngine != null) {
      return;
    }
    _holdEngine = engine;
    _holdDeckId = deckId;
    _session.begin(() async {
      try {
        await _runHold(() => engine.beginCueHold(deckId: deckId));
      } on Object catch (_) {
        // No hold to release for this pair.
        _clearHold(engine, deckId);
        rethrow;
      }
    });
  }

  void _endHold() {
    final engine = _holdEngine;
    final deckId = _holdDeckId;
    if (engine == null || deckId == null) {
      return;
    }
    _session.end(() => _runRelease(engine, deckId));
  }

  /// Release bound to the hold's transport. Failures are rethrown so the
  /// session keeps the hold active, and flagged so a dispose that only
  /// awaited a swallowed queued end can still retry.
  Future<void> _runRelease(EngineTransport engine, int deckId) async {
    try {
      await engine.endCueHold(deckId: deckId);
      _lastReleaseFailed = false;
      _clearHold(engine, deckId);
    } on Object catch (e) {
      _lastReleaseFailed = true;
      _reportCueError(e);
      rethrow;
    }
  }

  void _clearHold(EngineTransport engine, int deckId) {
    if (identical(_holdEngine, engine) && _holdDeckId == deckId) {
      _holdEngine = null;
      _holdDeckId = null;
    }
  }

  /// Runs a hold command for the session: failures are surfaced and rethrown
  /// so the session keeps the hold active and a later release can retry.
  Future<void> _runHold(Future<void> Function() command) async {
    try {
      await command();
    } on Object catch (e) {
      _reportCueError(e);
      rethrow;
    }
  }

  void _setCue() {
    final engine = _engine;
    final deckId = widget.deckId;
    if (engine == null) {
      return;
    }
    unawaited(_setCuePoint(engine, deckId));
  }

  Future<void> _setCuePoint(EngineTransport engine, int deckId) async {
    try {
      await engine.setCuePoint(deckId: deckId);
    } on Object catch (e) {
      _reportCueError(e);
    }
  }

  void _reportCueError(Object error) {
    if (!mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$error'),
    );
  }

  @override
  void dispose() {
    final engine = _holdEngine;
    final deckId = _holdDeckId;
    // Fire-and-forget: dispose cannot await. The session retries once when
    // the queued release failed, so a transient engine error cannot leave the
    // deck auditioning with no control left.
    unawaited(_releaseOnDispose(engine, deckId));
    _engineListen?.close();
    super.dispose();
  }

  Future<void> _releaseOnDispose(EngineTransport? engine, int? deckId) async {
    if (engine == null || deckId == null) {
      return;
    }
    Future<void> release() async {
      await _session.disposeEnd(() => _runRelease(engine, deckId));
      if (_lastReleaseFailed) {
        throw StateError('cue release failed');
      }
    }

    try {
      await release();
    } on Object catch (e) {
      debugPrint('DeckCueControl: endCueHold failed during dispose: $e');
      try {
        await release();
      } on Object catch (e) {
        debugPrint(
          'DeckCueControl: endCueHold retry failed during dispose: $e',
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DeckCueButton(
      disabled: widget.disabled,
      onBeginHold: _beginHold,
      onEndHold: _endHold,
      onSetCue: _setCue,
    );
  }
}

class _DeckPlayButton extends ConsumerWidget {
  const new({required this.deckId, required this.disabled});

  final int deckId;
  final bool disabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final playing = ref.watch(deckPlayingProvider(deckId));
    final playLabel = playing ? 'Pause' : 'Play';
    // Outline transport chrome (website stem play parity): hairline + fg at
    // rest; accent border/icon on hover, press, and while playing.
    return AppTooltip(
      tip: playLabel,
      child: MTappable(
        selected: playing,
        onPress: disabled
            ? null
            : () {
                final wantPlaying = !playing;
                unawaited(_setPlaying(context, ref, deckId, wantPlaying));
              },
        semanticsLabel: playLabel,
        builder: (context, state) {
          final accented = state.active;
          final borderColor = accented
              ? theme.colors.accent
              : theme.colors.border;
          final fg = accented ? theme.colors.accent : theme.colors.foreground;
          // Soft solid teal only while playing — hover stays outline-only.
          final fill = playing ? theme.colors.accentTint : theme.colors.card;
          return Opacity(
            opacity: state.disabled ? 0.5 : 1,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: fill,
                border: Border.all(
                  color: borderColor,
                  width: theme.style.borderWidth,
                ),
                borderRadius: deckTransportRadius(theme),
              ),
              child: ConstrainedBox(
                constraints: const BoxConstraints(minHeight: 36),
                child: Center(
                  // Optical nudge for the play triangle
                  // (site translate-x-px).
                  child: Transform.translate(
                    offset: playing ? Offset.zero : const Offset(1, 0),
                    child: Icon(
                      playing ? LucideIcons.pause : LucideIcons.play,
                      size: 18,
                      color: fg,
                    ),
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

Future<void> _engineCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}

Future<void> _setPlaying(
  BuildContext context,
  WidgetRef ref,
  int deckId,
  bool playing,
) async {
  await _engineCmd(context, () => setDeckPlaying(ref, deckId, playing));
}
