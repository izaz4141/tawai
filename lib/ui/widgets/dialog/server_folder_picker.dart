import 'package:flutter/material.dart';

import 'package:tawai/models/fs.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/api_service.dart';

/// A dialog that browses the **server's** filesystem and lets the user select
/// a folder. Used in remote/web mode, where a local `file_picker` dialog
/// would return a client-side path that does not exist on the server.
Future<String?> showServerFolderPicker(
  BuildContext context, {
  String? startPath,
}) {
  return showDialog<String>(
    context: context,
    builder: (context) => _ServerFolderPicker(startPath: startPath),
  );
}

class _ServerFolderPicker extends StatefulWidget {
  final String? startPath;

  const _ServerFolderPicker({this.startPath});

  @override
  State<_ServerFolderPicker> createState() => _ServerFolderPickerState();
}

class _ServerFolderPickerState extends State<_ServerFolderPicker> {
  String? _currentPath;
  List<FsEntry> _entries = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _currentPath = widget.startPath;
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    await _loadAt(_currentPath ?? '/');
  }

  Future<void> _loadAt(String path) async {
    final listing = await APIService.instance.listDirectory(path);
    if (!mounted) return;
    if (listing == null) {
      setState(() {
        _loading = false;
        _error = 'Unable to read folder "$path".';
      });
      return;
    }
    setState(() {
      _currentPath = listing.path;
      _entries = listing.entries;
      _loading = false;
      _error = null;
    });
  }

  void _enter(FsEntry entry) {
    if (!entry.isDir) return;
    _currentPath = entry.path;
    _load();
  }

  void _goUp() async {
    final current = _currentPath;
    if (current == null || current == '/') return;
    final listing = await APIService.instance.listDirectory(current);
    if (!mounted || listing?.parent == null) return;
    _currentPath = listing!.parent;
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: Text('Choose Folder', style: textTheme.titleLarge),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: _loading
            ? const SizedBox(
                height: 200,
                child: Center(child: CircularProgressIndicator()),
              )
            : Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          _currentPath ?? 'Select folder',
                          style: textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (_currentPath != null && _currentPath != '/')
                        IconButton(
                          tooltip: 'Go up',
                          icon: const Icon(Icons.arrow_upward),
                          onPressed: _goUp,
                        ),
                    ],
                  ),
                  if (_error != null) ...[
                    SizedBox(
                      height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                    ),
                    Text(
                      _error!,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.error,
                      ),
                    ),
                  ],
                  SizedBox(
                    height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                  ),
                  Flexible(
                    child: ListView.builder(
                      shrinkWrap: true,
                      itemCount: _entries.length,
                      itemBuilder: (context, i) {
                        final entry = _entries[i];
                        return ListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          leading: Icon(
                            Icons.folder_outlined,
                            color: colorScheme.primary,
                            size: AppTheme.iconMD * AppTheme.iconScale(context),
                          ),
                          title: Text(
                            entry.name,
                            style: textTheme.bodyMedium,
                            overflow: TextOverflow.ellipsis,
                          ),
                          onTap: () => _enter(entry),
                        );
                      },
                    ),
                  ),
                  if (_entries.isEmpty && _error == null)
                    Padding(
                      padding: EdgeInsets.symmetric(
                        vertical:
                            AppTheme.spaceMD * AppTheme.spaceScale(context),
                      ),
                      child: Text(
                        'No subfolders',
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                ],
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _currentPath == null
              ? null
              : () => Navigator.of(context).pop(_currentPath),
          child: const Text('Choose'),
        ),
      ],
    );
  }
}
