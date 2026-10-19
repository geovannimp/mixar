import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_hook_form/flutter_hook_form.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
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
/// Validators live in the schema so each field owns its rule. Only fields
/// the user has interacted with are validated on save, so a pre-existing
/// invalid value in an untouched field does not block unrelated edits.
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

/// Trimmed BPM text → positive finite double, or null when empty/invalid.
///
/// Single source of truth for BPM parsing so the validator and the save path
/// can never diverge.
double? parseBpm(String? raw) {
  final trimmed = raw?.trim() ?? '';
  if (trimmed.isEmpty) {
    return null;
  }
  final parsed = double.tryParse(trimmed);
  return parsed != null && parsed.isFinite && parsed > 0 ? parsed : null;
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
    return parseBpm(trimmed) != null ? null : 'BPM must be a positive number';
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
///
/// The store is a full replacement of the seven metadata columns, so a blank
/// field clears the stored value and untouched fields are sent back verbatim
/// (see `_TrackEditPanel.save`) — an edit never rewrites a field the user did
/// not touch.
Future<void> showTrackEditDrawer(
  BuildContext context,
  WidgetRef ref, {
  required String trackId,
}) async {
  try {
    final transport = await ref.read(libraryTransportProvider.future);
    // `ref` belongs to the caller's element, so it may only be used while that
    // element is mounted — check before every await resumes.
    if (!context.mounted) {
      return;
    }
    final track = await transport.getTrack(trackId: trackId);
    if (!context.mounted) {
      return;
    }
    if (track == null) {
      ref
          .read(appNoticeProvider.notifier)
          .setError('Track no longer exists in the library');
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
  } on Object catch (e) {
    // Same channel as the rest of the shell: AppNoticeBridge toasts it.
    if (context.mounted) {
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }
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

    final canSave = form.listen(
      TrackFormField.values.toSet(),
      (get) => form.hasChanged,
    );
    final saving = useState(false);
    final dismissed = useRef(false);
    useEffect(
      () =>
          () => dismissed.value = true,
      [],
    );

    Future<void> save() async {
      if (saving.value) {
        return;
      }
      // Validate only fields the user has interacted with; untouched fields
      // may carry pre-existing invalid values that should not block edits.
      for (final field in TrackFormField.values) {
        if (form.isDirty({field}) && !form.validateField(field)) {
          ref
              .read(appNoticeProvider.notifier)
              .setError('Fix the highlighted fields');
          return;
        }
      }
      // Captured before the await: after it, `context`/`ref` may be unusable.
      final navigator = Navigator.of(context);
      final route = ModalRoute.of(context);
      // A field the user never touched writes back exactly what was loaded,
      // so an edit cannot rewrite or clear it (and an untouched BPM keeps its
      // stored precision instead of being re-parsed from the prefill text).
      String? edited(TrackFormField f, String? stored) =>
          form.isDirty({f}) ? form.getValue<String>(f) : stored;
      saving.value = true;
      try {
        // Raw strings: trimming and key normalization happen Rust-side.
        await transport.updateTrackMetadata(
          trackId: trackId,
          title: edited(TrackFormField.title, track.title),
          artist: edited(TrackFormField.artist, track.artist),
          album: edited(TrackFormField.album, track.album),
          genre: edited(TrackFormField.genre, track.genre),
          bpm: form.isDirty({TrackFormField.bpm})
              ? parseBpm(form.getValue(TrackFormField.bpm))
              : track.bpm,
          key: edited(TrackFormField.key, track.key),
          isrc: edited(TrackFormField.isrc, track.isrc),
        );
        if (dismissed.value) {
          return;
        }
        ref
          ..invalidate(collectionTracksProvider)
          ..invalidate(driveResolvedByPathProvider);
        // A dismissed route is still `mounted` while it animates out, so
        // `isCurrent` — not `context.mounted` — is the guard that stops a
        // second pop from taking the route underneath the drawer.
        if (route == null || route.isCurrent) {
          navigator.pop();
        }
      } on Object catch (e) {
        if (!dismissed.value) {
          ref.read(appNoticeProvider.notifier).setError('$e');
        }
      } finally {
        if (!dismissed.value) {
          saving.value = false;
        }
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
                    _buildField('Title', TrackFormField.title),
                    const SizedBox(height: 12),
                    _buildField('Artist', TrackFormField.artist),
                    const SizedBox(height: 12),
                    _buildField('Album', TrackFormField.album),
                    const SizedBox(height: 12),
                    _buildField('Genre', TrackFormField.genre),
                    const SizedBox(height: 12),
                    _buildField(
                      'BPM',
                      TrackFormField.bpm,
                      keyboardType: TextInputType.number,
                    ),
                    const SizedBox(height: 12),
                    _buildField(
                      'Key',
                      TrackFormField.key,
                      hint: 'e.g. Am, or Camelot 8A',
                    ),
                    const SizedBox(height: 12),
                    _buildField(
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
                onPress: canSave && !saving.value ? save : null,
                child: const Text('Save'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildField(
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

  /// `128.0` renders as `128`; everything else renders as its shortest exact
  /// text, so a stored `124.035` shows as `124.035` and round-trips through
  /// the field without being committed as a rounded value. Extreme magnitudes
  /// fall back to fixed-point (never scientific) — `double.tryParse` reads
  /// both forms, and neither can throw on a non-finite input here.
  static String _formatBpm(double? bpm) {
    if (bpm == null || !bpm.isFinite) {
      return '';
    }
    if (bpm == bpm.roundToDouble()) {
      return bpm.round().toString();
    }
    final shortest = bpm.toString();
    return shortest.contains('e') || shortest.contains('E')
        ? bpm.toStringAsFixed(12)
        : shortest;
  }
}
