import 'package:flutter/material.dart';

import 'package:tawai/models/fs.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/api_service.dart';

/// A dialog that browses the **server's** filesystem and lets the user select
/// a folder. Used in remote/web mode, where a local `file_picker` dialog
/// would return a client-side path that does not exist on the server.
class FolderPickerDialog extends StatefulWidget {
  final String? startPath;

  const FolderPickerDialog({super.key, this.startPath});

  static Future<String?> show(BuildContext context, {String? startPath}) {
    return showDialog<String>(
      context: context,
      builder: (context) => FolderPickerDialog(startPath: startPath),
    );
  }

  @override
  State<FolderPickerDialog> createState() => _FolderPickerDialogState();
}

class _FolderPickerDialogState extends State<FolderPickerDialog> {
  final TextEditingController _pathController = TextEditingController();
  String? _currentPath;
  List<FsEntry> _entries = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _currentPath = widget.startPath;
    _pathController.text = widget.startPath ?? '';
    _load();
  }

  @override
  void dispose() {
    _pathController.dispose();
    super.dispose();
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
      _pathController.text = listing.path;
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

  void _submitPath(String value) {
    final path = value.trim();
    if (path.isEmpty) return;
    _loadAt(path);
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final spaceScale = AppTheme.spaceScale(context);
    final iconScale = AppTheme.iconScale(context);

    return AlertDialog(
      title: Row(
        children: [
          CircleAvatar(
            radius: AppTheme.iconLG * iconScale / 2,
            backgroundColor: colorScheme.primaryContainer,
            foregroundColor: colorScheme.onPrimaryContainer,
            child: Icon(
              Icons.folder_rounded,
              size: AppTheme.iconMD * iconScale,
            ),
          ),
          SizedBox(width: AppTheme.spaceMD * spaceScale),
          Text('Choose Folder'),
        ],
      ),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: _loading
            ? const SizedBox(
                height: 200,
                child: Center(child: CircularProgressIndicator()),
              )
            : Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  TextField(
                    controller: _pathController,
                    style: textTheme.bodyMedium?.copyWith(
                      fontFamily: 'monospace',
                      color: colorScheme.onSurface,
                    ),
                    decoration: InputDecoration(
                      hintText: 'Type a path',
                      isDense: true,
                      prefixIcon: Icon(
                        Icons.folder_open_rounded,
                        color: colorScheme.primary,
                      ),
                      suffixIcon: _currentPath != null && _currentPath != '/'
                          ? IconButton(
                              tooltip: 'Go up',
                              icon: const Icon(Icons.arrow_upward),
                              onPressed: _goUp,
                            )
                          : null,
                      filled: true,
                      fillColor: colorScheme.surfaceContainerHighest,
                    ),
                    onSubmitted: _submitPath,
                  ),
                  if (_error != null) ...[
                    SizedBox(height: AppTheme.spaceSM * spaceScale),
                    Container(
                      padding: EdgeInsets.all(AppTheme.spaceSM * spaceScale),
                      decoration: BoxDecoration(
                        color: colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(
                          AppTheme.radiusMD * AppTheme.radiusScale(context),
                        ),
                      ),
                      child: Row(
                        children: [
                          Icon(
                            Icons.error_outline_rounded,
                            size: AppTheme.iconSM * iconScale,
                            color: colorScheme.onErrorContainer,
                          ),
                          SizedBox(width: AppTheme.spaceSM * spaceScale),
                          Expanded(
                            child: Text(
                              _error!,
                              style: textTheme.bodySmall?.copyWith(
                                color: colorScheme.onErrorContainer,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                  SizedBox(height: AppTheme.spaceMD * spaceScale),
                  if (_entries.isEmpty && _error == null)
                    Padding(
                      padding: EdgeInsets.symmetric(
                        vertical: AppTheme.spaceXL * spaceScale,
                      ),
                      child: Column(
                        children: [
                          Icon(
                            Icons.folder_open_rounded,
                            size: AppTheme.iconXL * iconScale,
                            color: colorScheme.onSurfaceVariant.withAlpha(120),
                          ),
                          SizedBox(height: AppTheme.spaceSM * spaceScale),
                          Text(
                            'No subfolders',
                            style: textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    )
                  else
                    Flexible(
                      child: ConstrainedBox(
                        constraints: BoxConstraints(
                          maxHeight: AppTheme.dialogMaxHeight(context),
                        ),
                        child: ListView.builder(
                          shrinkWrap: true,
                          padding: EdgeInsets.only(
                            bottom: AppTheme.spaceXS * spaceScale,
                          ),
                          itemCount: _entries.length,
                          itemBuilder: (context, i) {
                            final entry = _entries[i];
                            return Padding(
                              padding: EdgeInsets.only(
                                bottom: AppTheme.spaceXS * spaceScale,
                              ),
                              child: _FolderRow(entry: entry, onTap: _enter),
                            );
                          },
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

class _FolderRow extends StatelessWidget {
  final FsEntry entry;
  final ValueChanged<FsEntry> onTap;

  const _FolderRow({required this.entry, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDir = entry.isDir;
    final iconScale = AppTheme.iconScale(context);
    final spaceScale = AppTheme.spaceScale(context);

    return Material(
      color: colorScheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(
        AppTheme.radiusMD * AppTheme.radiusScale(context),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: isDir ? () => onTap(entry) : null,
        child: Padding(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceSM * spaceScale,
            vertical: AppTheme.spaceSM * spaceScale,
          ),
          child: Row(
            children: [
              Container(
                padding: EdgeInsets.all(AppTheme.spaceXS * spaceScale),
                decoration: BoxDecoration(
                  color: isDir
                      ? colorScheme.primaryContainer.withAlpha(90)
                      : colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(
                    AppTheme.radiusSM * AppTheme.radiusScale(context),
                  ),
                ),
                child: Icon(
                  isDir
                      ? Icons.folder_rounded
                      : Icons.insert_drive_file_outlined,
                  size: AppTheme.iconSM * iconScale,
                  color: isDir
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
              ),
              SizedBox(width: AppTheme.spaceSM * spaceScale),
              Expanded(
                child: Text(
                  entry.name,
                  style: textTheme.bodyMedium?.copyWith(
                    color: isDir
                        ? colorScheme.onSurface
                        : colorScheme.onSurfaceVariant,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              if (isDir)
                Icon(
                  Icons.chevron_right_rounded,
                  size: AppTheme.iconMD * iconScale,
                  color: colorScheme.onSurfaceVariant,
                ),
            ],
          ),
        ),
      ),
    );
  }
}
