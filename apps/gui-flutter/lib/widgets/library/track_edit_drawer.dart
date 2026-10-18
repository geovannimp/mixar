import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_hook_form/flutter_hook_form.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Drawer width; clamped to the viewport on narrow windows.
const trackEditDrawerWidth = 420.0;

/// Fields of the track edit drawer, in form order.
///
/// Validators live in the schema so each field owns its rule and the whole
/// surface validates in one call.
enum TrackFormField implements FieldSchema<String> {
  title,
  artist,
  album,
  genre,
  bpm(validators: [BpmValidator()]),
  key(validators: [KeyNotationValidator()]),
  isrc;

  new({this.validators});

  @override
  final List<Validator<String>>? validators;
}

/// `BPM` must be a positive finite number, or empty to clear it.
class BpmValidator extends FieldValidator<String> {
  const new() : super(errorCode: 'invalid_bpm');

  @override
  FieldValidatorFn<String> get validator => (value) {
    final trimmed = value?.trim() ?? '';
    if (trimmed.isEmpty) {
      return null;
    }
    final parsed = double.tryParse(trimmed);
    if (parsed == null || !parsed.isFinite || parsed <= 0) {
      return 'BPM must be a positive number';
    }
    return null;
  };
}

/// `Key` must be musical notation, or a Camelot/Open Key code the Rust write
/// path normalizes.
class KeyNotationValidator extends FieldValidator<String> {
  const new() : super(errorCode: 'invalid_key');

  static final _musical = RegExp(r'^[A-Ga-g][#b]?m?$');

  @override
  FieldValidatorFn<String> get validator => (value) {
    final trimmed = value?.trim() ?? '';
    if (trimmed.isEmpty) {
      return null;
    }
    if (_musical.hasMatch(trimmed) || camelotToMusical(trimmed) != null) {
      return null;
    }
    return 'Use a musical key (Am) or Camelot code (8A)';
  };
}

/// Modal right-edge drawer editing every user-editable metadata field of a
/// track. Edits are library-DB only: file tags are not rewritten.
Future<void> showTrackEditDrawer(
  BuildContext context,
  WidgetRef ref, {
  required String trackId,
}) async {
  final transport = await ref.read(libraryTransportProvider.future);
  final track = await transport.getTrack(trackId: trackId);
  if (!context.mounted || track == null) {
    return;
  }
  await showGeneralDialog<void>(
    context: context,
    barrierDismissible: true,
    barrierLabel: 'Edit track',
    transitionBuilder: (context, animation, _, child) {
      return SlideTransition(
        position: Tween<Offset>(begin: const Offset(1, 0), end: Offset.zero)
            .animate(
              CurvedAnimation(parent: animation, curve: Curves.easeOutCubic),
            ),
        child: child,
      );
    },
    pageBuilder: (ctx, _, _) {
      return Align(
        alignment: Alignment.centerRight,
        child: SizedBox(
          width: math.min(trackEditDrawerWidth, MediaQuery.sizeOf(ctx).width),
          height: double.infinity,
          child: _TrackEditPanel(
            transport: transport,
            trackId: trackId,
            track: track,
          ),
        ),
      );
    },
  );
}

class _TrackEditPanel extends HookConsumerWidget {
  const new({
    required this.transport,
    required this.trackId,
    required this.track,
  });

  final LibraryTransport transport;
  final String trackId;
  final LibraryTrackSummary track;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final title = track.title ?? track.displayName;
    final artist = track.artist;

    // Seeded from the row; the form's own change tracking drives Save
    // enablement, so reverting a field disables it again.
    final form = useForm<TrackFormField>(
      initialValues: {
        TrackFormField.title: track.title ?? '',
        TrackFormField.artist: track.artist ?? '',
        TrackFormField.album: track.album ?? '',
        TrackFormField.genre: track.genre ?? '',
        TrackFormField.bpm: _formatBpm(track.bpm),
        TrackFormField.key: track.key ?? '',
        TrackFormField.isrc: track.isrc ?? '',
      },
    );

    // Rebuild on any field change so Save reflects the live dirty state.
    final canSave = form.listen(
      TrackFormField.values.toSet(),
      (get) => form.hasChanged,
    );

    Future<void> save() async {
      if (!form.validate()) {
        ref
            .read(appNoticeProvider.notifier)
            .setError('Fix the highlighted fields');
        return;
      }
      try {
        // Raw strings: trimming and key normalization happen Rust-side.
        await transport.updateTrackMetadata(
          trackId: trackId,
          title: form.getValue(TrackFormField.title),
          artist: form.getValue(TrackFormField.artist),
          album: form.getValue(TrackFormField.album),
          genre: form.getValue(TrackFormField.genre),
          bpm: _parseBpm(form.getValue(TrackFormField.bpm)),
          key: form.getValue(TrackFormField.key),
          isrc: form.getValue(TrackFormField.isrc),
        );
        ref
          ..invalidate(collectionTracksProvider)
          ..invalidate(driveResolvedByPathProvider);
        if (context.mounted) {
          Navigator.of(context).pop();
        }
      } on Object catch (e) {
        ref.read(appNoticeProvider.notifier).setError('$e');
      }
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        // Square corners, flush to the screen edge: only the inner edge rules.
        border: Border(
          left: BorderSide(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
      ),
      child: HookedForm(
        form: form,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          title,
                          style: theme.typography.body.md.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        if (artist != null && artist.isNotEmpty) ...[
                          const SizedBox(height: 4),
                          Text(
                            artist,
                            style: theme.typography.body.sm.copyWith(
                              color: theme.colors.mutedForeground,
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                  AppButton.icon(
                    variant: MixarButtonVariant.ghost,
                    size: MixarButtonSize.sm,
                    semanticsLabel: 'Close',
                    onPress: () => Navigator.of(context).pop(),
                    child: const Icon(LucideIcons.x, size: 20),
                  ),
                ],
              ),
            ),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _field('Title', TrackFormField.title),
                    const SizedBox(height: 12),
                    _field('Artist', TrackFormField.artist),
                    const SizedBox(height: 12),
                    _field('Album', TrackFormField.album),
                    const SizedBox(height: 12),
                    _field('Genre', TrackFormField.genre),
                    const SizedBox(height: 12),
                    _field(
                      'BPM',
                      TrackFormField.bpm,
                      keyboardType: TextInputType.number,
                    ),
                    const SizedBox(height: 12),
                    _field(
                      'Key',
                      TrackFormField.key,
                      hint: 'e.g. Am, or Camelot 8A',
                    ),
                    const SizedBox(height: 12),
                    _field(
                      'ISRC',
                      TrackFormField.isrc,
                      hint: 'International Standard Recording Code',
                    ),
                  ],
                ),
              ),
            ),
            // Outside the scroll area so Save stays reachable in short windows.
            // Dismissal is the header close button or a barrier tap; no Cancel.
            Padding(
              padding: const EdgeInsets.all(16),
              child: AppButton(
                onPress: canSave ? save : null,
                child: const Text('Save'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _field(
    String label,
    TrackFormField field, {
    String? hint,
    TextInputType? keyboardType,
  }) {
    return HookedFormField<String, TrackFormField>(
      fieldHook: field,
      autovalidateMode: AutovalidateMode.onUserInteraction,
      builder: (value, onChanged, error) => MixarInput(
        label: Text(label),
        hint: hint,
        keyboardType: keyboardType,
        initialValue: value,
        onChanged: onChanged,
        errorText: error,
      ),
    );
  }

  /// `128.0` renders as `128`; fractional BPMs keep their decimals.
  static String _formatBpm(double? bpm) {
    if (bpm == null) {
      return '';
    }
    return bpm == bpm.roundToDouble() ? bpm.round().toString() : '$bpm';
  }

  /// Empty clears the field; the schema guarantees a positive finite value.
  static double? _parseBpm(String? raw) {
    final trimmed = raw?.trim() ?? '';
    return trimmed.isEmpty ? null : double.tryParse(trimmed);
  }
}
