import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

enum CreateCollectionType { folder, playlist }

const _typeFolder = 'folder';
const _typePlaylist = 'playlist';

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
  });

  final String name;
  final CreateCollectionType type;
  final String? folderPath;
  final bool scanSubfolders;
  final bool sortable;
}

Future<CreateCollectionResult?> showCreateCollectionDialog(
  BuildContext context, {
  CreateCollectionInput input = const CreateCollectionInput(),
}) {
  return showMixarDialog<CreateCollectionResult?>(
    context: context,
    builder: (context) => _CreateCollectionDialogBody(input: input),
  );
}

class _CreateCollectionDialogBody extends StatefulWidget {
  const new({required this.input});

  final CreateCollectionInput input;

  @override
  State<_CreateCollectionDialogBody> createState() =>
      _CreateCollectionDialogBodyState();
}

class _CreateCollectionDialogBodyState
    extends State<_CreateCollectionDialogBody> {
  late final TextEditingController _nameController;
  late final TextEditingController _folderPathController;
  late String _typeKey = widget.input.historySessionId == null
      ? _typeFor(widget.input.initialType)
      : _typePlaylist;
  late bool _scanSubfolders = widget.input.initialScanSubfolders;
  late bool _sortable = widget.input.initialSortable;

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

  CreateCollectionType get _type =>
      _typeKey == _typeFolder ? CreateCollectionType.folder : .playlist;

  bool get _canCreate {
    if (_nameController.text.trim().isEmpty) {
      return false;
    }
    return switch (_type) {
      CreateCollectionType.folder =>
        _folderPathController.text.trim().isNotEmpty,
      CreateCollectionType.playlist => true,
    };
  }

  static String _typeFor(CreateCollectionType type) {
    return switch (type) {
      CreateCollectionType.folder => _typeFolder,
      CreateCollectionType.playlist => _typePlaylist,
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
    final path = await FilePicker.platform.getDirectoryPath();
    if (path == null || !mounted) {
      return;
    }
    _applyBrowsedFolder(path);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Create collection',
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 16),
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
                value: _typeKey,
                options: const [_typeFolder, _typePlaylist],
                labelBuilder: (value) => switch (value) {
                  _typeFolder => 'Folder',
                  _ => 'Playlist',
                },
                subtitleBuilder: (value) => switch (value) {
                  _typeFolder => 'Import audio files from a directory on disk.',
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
                    SettingsToggle(
                      label: 'Scan subfolders',
                      value: _scanSubfolders,
                      onChanged: (value) =>
                          setState(() => _scanSubfolders = value),
                    ),
                  ] else ...[
                    SettingsToggle(
                      label: 'Sortable',
                      value: _sortable,
                      onChanged: (value) => setState(() => _sortable = value),
                    ),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          Row(
            spacing: 8,
            children: [
              AppButton(
                variant: .outline,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Cancel'),
              ),
              AppButton(
                onPress: _canCreate
                    ? () {
                        Navigator.of(context).pop(
                          CreateCollectionResult(
                            name: _nameController.text.trim(),
                            type: _type,
                            folderPath: _type == CreateCollectionType.folder
                                ? _folderPathController.text.trim()
                                : null,
                            scanSubfolders: _scanSubfolders,
                            sortable: _sortable,
                          ),
                        );
                      }
                    : null,
                child: const Text('Create'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
