import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/widgets/library/library_form_drawer.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

enum CreateCollectionType { folder, playlist, smartPlaylist }

const _typeFolder = 'folder';
const _typePlaylist = 'playlist';
const String _typeSmart = kSmartPlaylistKind;

class CreateCollectionInput {
  const new({
    this.initialName = '',
    this.initialType = CreateCollectionType.folder,
    this.initialFolderPath,
    this.initialScanSubfolders = true,
    this.initialSortable = true,
    this.historySessionId,
  });

  final String initialName;
  final CreateCollectionType initialType;
  final String? initialFolderPath;
  final bool initialScanSubfolders;
  final bool initialSortable;

  /// When set, only playlist creation is offered and session tracks are copied in.
  final String? historySessionId;
}

class CreateCollectionResult {
  const new({
    required this.name,
    required this.type,
    required this.folderPath,
    required this.scanSubfolders,
    required this.sortable,
    this.rules,
  });

  final String name;
  final CreateCollectionType type;
  final String? folderPath;
  final bool scanSubfolders;
  final bool sortable;

  /// Smart playlists only: the rule document to create with.
  final SmartPlaylistRules? rules;
}

/// Drawer width; clamped to the viewport on narrow windows.
const createCollectionDrawerWidth = 480.0;

/// Create a collection in a right-edge drawer, like the track editor.
///
/// Returns `null` when the drawer is dismissed without creating anything.
Future<CreateCollectionResult?> showCreateCollectionDrawer(
  BuildContext context, {
  CreateCollectionInput input = const CreateCollectionInput(),
}) {
  return showLibraryFormDrawer<CreateCollectionResult>(
    context,
    barrierLabel: 'Create collection',
    width: createCollectionDrawerWidth,
    builder: (context) => _CreateCollectionDrawerBody(input: input),
  );
}

class _CreateCollectionDrawerBody extends StatefulWidget {
  const new({required this.input});

  final CreateCollectionInput input;

  @override
  State<_CreateCollectionDrawerBody> createState() =>
      _CreateCollectionDrawerBodyState();
}

class _CreateCollectionDrawerBodyState
    extends State<_CreateCollectionDrawerBody> {
  late final TextEditingController _nameController;
  late final TextEditingController _folderPathController;
  late String _typeKey = widget.input.historySessionId == null
      ? _typeFor(widget.input.initialType)
      : _typePlaylist;
  late bool _scanSubfolders = widget.input.initialScanSubfolders;
  late bool _sortable = widget.input.initialSortable;
  SmartPlaylistRules _rules = const SmartPlaylistRules.empty();

  /// Last name this drawer derived from a preset. A newer preset replaces a
  /// derived name, never one the user typed, mirroring how browsing a folder
  /// keeps a name the user chose.
  String? _derivedName;

  @override
  void initState() {
    super.initState();
    final initialPath = widget.input.initialFolderPath ?? '';
    final initialName = widget.input.initialName.isNotEmpty
        ? widget.input.initialName
        : (_collectionNameFromFolder(initialPath) ?? '');
    _nameController = TextEditingController(text: initialName);
    _folderPathController = TextEditingController(text: initialPath);
  }

  @override
  void dispose() {
    _nameController.dispose();
    _folderPathController.dispose();
    super.dispose();
  }

  bool get _fromHistory => widget.input.historySessionId != null;

  CreateCollectionType get _type => switch (_typeKey) {
    _typeFolder => CreateCollectionType.folder,
    _typeSmart => CreateCollectionType.smartPlaylist,
    _ => CreateCollectionType.playlist,
  };

  /// Name an unnamed smart playlist after the preset it starts from.
  void _applyPresetName(SmartPlaylistRulesPreset preset) {
    final current = _nameController.text;
    if (current.trim().isEmpty || current == _derivedName) {
      _nameController.text = preset.name;
      _derivedName = preset.name;
    }
  }

  /// First problem with the rule document, or a nudge while it has none.
  String? get _rulesHint {
    if (_rules.root.rules.isEmpty) {
      return 'Add at least one rule.';
    }
    return _rules.validate();
  }

  bool get _canCreate {
    if (_nameController.text.trim().isEmpty) {
      return false;
    }
    return switch (_type) {
      CreateCollectionType.folder =>
        _folderPathController.text.trim().isNotEmpty,
      CreateCollectionType.playlist => true,
      // The library rejects an invalid document after the drawer closes, which
      // would throw the rule tree away: block it before that can happen.
      CreateCollectionType.smartPlaylist =>
        _rules.root.rules.isNotEmpty && _rules.validate() == null,
    };
  }

  static String _typeFor(CreateCollectionType type) {
    return switch (type) {
      CreateCollectionType.folder => _typeFolder,
      CreateCollectionType.playlist => _typePlaylist,
      CreateCollectionType.smartPlaylist => _typeSmart,
    };
  }

  static String? _collectionNameFromFolder(String path) {
    final trimmed = path.trim();
    if (trimmed.isEmpty) {
      return null;
    }
    final normalized = trimmed.replaceAll(r'\', '/');
    final slash = normalized.lastIndexOf('/');
    final name = slash >= 0 ? normalized.substring(slash + 1) : normalized;
    return name.isEmpty ? null : name;
  }

  void _applyBrowsedFolder(String path) {
    final previousDerived = _collectionNameFromFolder(
      _folderPathController.text,
    );
    final nextName = _collectionNameFromFolder(path);
    _folderPathController.text = path;
    final currentName = _nameController.text;
    if (currentName.trim().isEmpty || currentName == (previousDerived ?? '')) {
      _nameController.text = nextName ?? '';
    }
    setState(() {});
  }

  Future<void> _browseFolder() async {
    final path = await FilePicker.getDirectoryPath();
    if (path == null || !mounted) {
      return;
    }
    _applyBrowsedFolder(path);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LibraryFormDrawerShell(
      title: 'Create collection',
      action: AppButton(
        onPress: _canCreate ? _submit : null,
        child: const Text('Create'),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SettingsField(
            label: 'Name',
            child: MixarInput(
              controller: _nameController,
              onChanged: (_) => setState(() {}),
            ),
          ),
          if (!_fromHistory) ...[
            const SizedBox(height: 12),
            SettingsField(
              label: 'Type',
              child: MixarSelect<String>(
                dialogTitle: 'Type',
                value: _typeKey,
                options: const [_typeFolder, _typePlaylist, _typeSmart],
                labelBuilder: (value) => switch (value) {
                  _typeFolder => 'Folder',
                  _typeSmart => 'Smart playlist',
                  _ => 'Playlist',
                },
                subtitleBuilder: (value) => switch (value) {
                  _typeFolder => 'Import audio files from a directory on disk.',
                  _typeSmart => 'Rules that pick tracks automatically.',
                  _ => 'A manual track list in your library collections.',
                },
                onChanged: (value) => setState(() => _typeKey = value),
              ),
            ),
          ],
          const SizedBox(height: 12),
          DecoratedBox(
            decoration: BoxDecoration(
              border: Border(left: BorderSide(color: theme.colors.border)),
            ),
            child: Padding(
              padding: const EdgeInsets.only(left: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 16,
                children: [
                  if (_type == CreateCollectionType.folder) ...[
                    SettingsField(
                      label: 'Folder path',
                      child: IntrinsicHeight(
                        child: Row(
                          spacing: 8,
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            Expanded(
                              child: MixarInput(
                                hint: 'Choose a folder on disk',
                                controller: _folderPathController,
                                onChanged: (_) => setState(() {}),
                              ),
                            ),
                            AppButton(
                              variant: .outline,
                              onPress: _browseFolder,
                              child: const Text('Browse…'),
                            ),
                          ],
                        ),
                      ),
                    ),
                    SettingsCheckbox(
                      label: 'Scan subfolders',
                      value: _scanSubfolders,
                      onChanged: (value) =>
                          setState(() => _scanSubfolders = value),
                    ),
                  ] else if (_type == CreateCollectionType.smartPlaylist) ...[
                    SettingsField(
                      label: 'Rules',
                      hint: _rulesHint,
                      child: SmartRulesEditor(
                        initial: _rules,
                        onPresetApplied: _applyPresetName,
                        onChanged: (next) => setState(() => _rules = next),
                      ),
                    ),
                  ] else ...[
                    SettingsCheckbox(
                      label: 'Sortable',
                      value: _sortable,
                      onChanged: (value) => setState(() => _sortable = value),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// Close the drawer with the collection to create.
  void _submit() {
    Navigator.of(context).pop(
      CreateCollectionResult(
        name: _nameController.text.trim(),
        type: _type,
        folderPath: _type == CreateCollectionType.folder
            ? _folderPathController.text.trim()
            : null,
        scanSubfolders: _scanSubfolders,
        sortable: _sortable,
        rules: _type == CreateCollectionType.smartPlaylist ? _rules : null,
      ),
    );
  }
}
