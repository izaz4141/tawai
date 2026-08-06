import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/helper.dart';
import 'package:tawai/utils/settings.dart';

class RenamerPage extends StatefulWidget {
  const RenamerPage({super.key});

  @override
  State<RenamerPage> createState() => _RenamerPageState();
}

enum _Phase { idle, loading, error, result }

class _RenamerPageState extends State<RenamerPage> {
  _Phase _phase = _Phase.idle;
  String? _error;

  final _patternController = TextEditingController();
  List<LibrarySourceInfo> _sources = [];
  String? _selectedSourceId;

  List<RenamePreview> _previews = [];
  Set<int> _selectedIndices = {};
  int _conformingCount = 0;

  @override
  void initState() {
    super.initState();
    _patternController.text = SettingsManager.namingPattern.value;
    _loadSources();
  }

  @override
  void dispose() {
    _patternController.dispose();
    super.dispose();
  }

  bool get _allSelected =>
      _previews.isNotEmpty && _selectedIndices.length == _previews.length;

  // ---------------------------------------------------------------------------
  // Data
  // ---------------------------------------------------------------------------

  Future<void> _loadSources() async {
    final userId = SettingsManager.currentUserId.value;
    if (userId == null || userId.isEmpty) return;
    try {
      final sources = await BridgeService.instance.listEditableSources(userId);
      if (!mounted) return;
      setState(() => _sources = sources);
    } catch (_) {}
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  Future<void> _previewRename() async {
    setState(() {
      _phase = _Phase.loading;
      _error = null;
    });
    try {
      final response = await BridgeService.instance.batchRenamePreview(
        filePaths: const [],
        pattern: _patternController.text,
        sourceId: _selectedSourceId,
      );
      if (!mounted) return;

      final allPreviews = response?.previews ?? [];
      if (allPreviews.isEmpty) {
        setState(() {
          _phase = _Phase.idle;
          _error = response?.error ?? 'No tracks found in selected source';
        });
        return;
      }

      final conforming = <RenamePreview>[];
      final nonConforming = <RenamePreview>[];

      for (final p in allPreviews) {
        if (p.filePath == p.expectedPath) {
          conforming.add(p);
        } else {
          nonConforming.add(p);
        }
      }

      setState(() {
        _previews = nonConforming;
        _selectedIndices = Set.from(Iterable.generate(nonConforming.length));
        _conformingCount = conforming.length;
        _phase = _Phase.result;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _phase = _Phase.error;
      });
    }
  }

  Future<void> _applyRename() async {
    if (_selectedIndices.isEmpty) return;
    setState(() => _phase = _Phase.loading);

    final selectedPaths = _selectedIndices
        .map((i) => _previews[i].filePath)
        .toList();

    try {
      await BridgeService.instance.batchRenameApply(
        filePaths: selectedPaths,
        pattern: _patternController.text,
      );
      if (!mounted) return;
      AppSnackBar.show(
        context,
        'Renamed ${selectedPaths.length} file(s)',
        type: SnackType.success,
      );
      setState(() {
        _previews = [];
        _selectedIndices = {};
        _conformingCount = 0;
        _phase = _Phase.idle;
      });
    } catch (e) {
      if (!mounted) return;
      AppSnackBar.show(context, e.toString(), type: SnackType.error);
      setState(() => _phase = _Phase.result);
    }
  }

  void _toggleSelectAll() {
    setState(() {
      if (_allSelected) {
        _selectedIndices = {};
      } else {
        _selectedIndices = Set.from(Iterable.generate(_previews.length));
      }
    });
  }

  void _clearResults() {
    _phase = _Phase.idle;
    _error = null;
    _previews = [];
    _selectedIndices = {};
    _conformingCount = 0;
  }

  // ---------------------------------------------------------------------------
  // Build
  // ---------------------------------------------------------------------------

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(
          'File Renamer',
          style: isDesktop ? textTheme.titleLarge : null,
        ),
      ),
      body: Column(
        children: [
          _buildHeader(),
          const Divider(height: 1),
          Expanded(child: _buildBody()),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // Header
  // ---------------------------------------------------------------------------

  Widget _buildHeader() {
    final scale = AppTheme.spaceScale(context);
    return Padding(
      padding: EdgeInsets.all(AppTheme.spaceMD * scale),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildSourceDropdown(),
          SizedBox(height: AppTheme.spaceSM * scale),
          _buildPatternField(),
          SizedBox(height: AppTheme.spaceSM * scale),
          _buildActionButton(),
        ],
      ),
    );
  }

  Widget _buildSourceDropdown() {
    final colors = Theme.of(context).colorScheme;
    return DropdownButtonFormField<String?>(
      value: _selectedSourceId,
      decoration: InputDecoration(
        labelText: 'Source',
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppTheme.radiusMD),
        ),
        contentPadding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceMD,
          vertical: AppTheme.spaceSM,
        ),
        isDense: true,
      ),
      items: [
        DropdownMenuItem(
          value: null,
          child: Row(
            children: [
              Icon(
                Icons.library_music,
                size: AppTheme.iconMD,
                color: colors.primary,
              ),
              SizedBox(width: AppTheme.spaceSM),
              const Text('All Library'),
            ],
          ),
        ),
        ..._sources.map(
          (src) => DropdownMenuItem(
            value: src.id,
            child: Row(
              children: [
                Icon(
                  Icons.folder,
                  size: AppTheme.iconMD,
                  color: colors.primary,
                ),
                SizedBox(width: AppTheme.spaceSM),
                Text(src.name, overflow: TextOverflow.ellipsis),
              ],
            ),
          ),
        ),
      ],
      onChanged: (v) {
        setState(() {
          _selectedSourceId = v;
          _clearResults();
        });
      },
    );
  }

  Widget _buildPatternField() {
    return TextField(
      controller: _patternController,
      decoration: const InputDecoration(
        labelText: 'Naming Pattern',
        helperText: '{artist} {album} {title} {track_padded} {year}',
        border: OutlineInputBorder(),
        isDense: true,
      ),
    );
  }

  Widget _buildActionButton() {
    final loading = _phase == _Phase.loading;
    return FilledButton.icon(
      onPressed: loading ? null : _previewRename,
      icon: loading
          ? SizedBox(
              width: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              child: const CircularProgressIndicator(strokeWidth: 2),
            )
          : Icon(
              Icons.preview,
              size: AppTheme.iconSM * AppTheme.iconScale(context),
            ),
      label: Text(loading ? 'Loading...' : 'Preview'),
    );
  }

  // ---------------------------------------------------------------------------
  // Body
  // ---------------------------------------------------------------------------

  Widget _buildBody() {
    switch (_phase) {
      case _Phase.idle:
        return _buildEmptyState();
      case _Phase.loading:
        return const Center(child: CircularProgressIndicator());
      case _Phase.error:
        return _buildErrorState();
      case _Phase.result:
        return _buildResults();
    }
  }

  Widget _buildEmptyState() {
    final colors = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.drive_file_rename_outline,
            size: AppTheme.iconXXL * AppTheme.iconScale(context),
            color: colors.onSurfaceVariant,
          ),
          SizedBox(height: AppTheme.spaceMD),
          Text(
            'Select a source and tap Preview',
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(color: colors.onSurfaceVariant),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorState() {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(AppTheme.spaceMD),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.error_outline,
              size: AppTheme.iconXL * AppTheme.iconScale(context),
              color: colors.error,
            ),
            SizedBox(height: AppTheme.spaceSM),
            Text(_error ?? 'Unknown error', style: textTheme.bodyMedium),
            SizedBox(height: AppTheme.spaceMD),
            FilledButton.icon(
              onPressed: _previewRename,
              icon: Icon(
                Icons.refresh,
                size: AppTheme.iconSM * AppTheme.iconScale(context),
              ),
              label: const Text('Retry'),
            ),
          ],
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // Results
  // ---------------------------------------------------------------------------

  Widget _buildResults() {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    if (_previews.isEmpty && _conformingCount > 0) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.check_circle,
              size: AppTheme.iconXXL * AppTheme.iconScale(context),
              color: colors.primary,
            ),
            SizedBox(height: AppTheme.spaceMD),
            Text(
              'All $_conformingCount file(s) already match the pattern',
              style: textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
          ],
        ),
      );
    }

    final needsRename = _previews.length;

    return Column(
      children: [
        Padding(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceMD,
            vertical: AppTheme.spaceSM,
          ),
          child: Row(
            children: [
              Checkbox(
                value: _allSelected,
                onChanged: (_) => _toggleSelectAll(),
              ),
              SizedBox(width: AppTheme.spaceSM),
              Expanded(
                child: Text(
                  needsRename > 0
                      ? '$needsRename file(s) to rename'
                      : 'No files to rename',
                  style: textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ),
              if (_conformingCount > 0)
                Padding(
                  padding: EdgeInsets.only(right: AppTheme.spaceSM),
                  child: Text(
                    '$_conformingCount already match',
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                ),
              FilledButton.tonalIcon(
                onPressed: _selectedIndices.isEmpty ? null : _applyRename,
                icon: Icon(
                  Icons.check,
                  size: AppTheme.iconSM * AppTheme.iconScale(context),
                ),
                label: Text('Apply (${_selectedIndices.length})'),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ListView.builder(
            padding: EdgeInsets.symmetric(
              horizontal: AppTheme.spaceSM,
              vertical: AppTheme.spaceXS,
            ),
            itemCount: _previews.length,
            itemBuilder: (context, index) {
              final p = _previews[index];
              return _RenamePreviewCard(
                preview: p,
                selected: _selectedIndices.contains(index),
                onToggle: (v) {
                  setState(() {
                    if (v) {
                      _selectedIndices.add(index);
                    } else {
                      _selectedIndices.remove(index);
                    }
                  });
                },
              );
            },
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Leaf widgets
// ---------------------------------------------------------------------------

class _RenamePreviewCard extends StatelessWidget {
  final RenamePreview preview;
  final bool selected;
  final ValueChanged<bool> onToggle;

  const _RenamePreviewCard({
    required this.preview,
    required this.selected,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isError = preview.expectedPath.startsWith('<error');

    final oldPath = pathStyling(preview.filePath);
    final newPath = isError ? '' : pathStyling(preview.expectedPath);

    return Card(
      margin: EdgeInsets.symmetric(vertical: AppTheme.spaceXS),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: isError ? null : () => onToggle(!selected),
        child: Padding(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceSM,
            vertical: AppTheme.spaceXS,
          ),
          child: Row(
            children: [
              IgnorePointer(
                child: Checkbox(value: selected, onChanged: (_) {}),
              ),
              SizedBox(width: AppTheme.spaceSM),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      oldPath,
                      style: textTheme.bodySmall?.copyWith(
                        decoration: isError ? TextDecoration.lineThrough : null,
                      ),
                      overflow: TextOverflow.ellipsis,
                      maxLines: 1,
                    ),
                    if (!isError) ...[
                      SizedBox(height: AppTheme.spaceXS),
                      Row(
                        children: [
                          Icon(
                            Icons.arrow_forward,
                            size: AppTheme.iconSM * AppTheme.iconScale(context),
                            color: colors.primary,
                          ),
                          SizedBox(width: AppTheme.spaceXS),
                          Expanded(
                            child: Text(
                              newPath,
                              style: textTheme.bodyMedium?.copyWith(
                                color: colors.primary,
                              ),
                              overflow: TextOverflow.ellipsis,
                              maxLines: 1,
                            ),
                          ),
                        ],
                      ),
                    ],
                  ],
                ),
              ),
              if (isError)
                Icon(
                  Icons.error,
                  size: AppTheme.iconMD * AppTheme.iconScale(context),
                  color: colors.error,
                ),
            ],
          ),
        ),
      ),
    );
  }
}
