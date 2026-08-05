import 'dart:async';
import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/scan_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_dropdown.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/list_text_field.dart';
import 'package:tawai/ui/widgets/dialog/naming_format.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/folder_picker.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/models/recommendation_source.dart';

class SettingsDiscoveryTab extends StatefulWidget {
  const SettingsDiscoveryTab({super.key});

  @override
  State<SettingsDiscoveryTab> createState() => _SettingsDiscoveryTabState();
}

class _SettingsDiscoveryTabState extends State<SettingsDiscoveryTab> {
  List<LibrarySourceInfo> _sources = [];

  @override
  void initState() {
    super.initState();
    _loadSources();
    ScanService.instance.acquire();
    ScanService.instance.isScanning.addListener(_onScanStateChanged);
  }

  @override
  void dispose() {
    ScanService.instance.isScanning.removeListener(_onScanStateChanged);
    ScanService.instance.release();
    super.dispose();
  }

  void _onScanStateChanged() {
    if (!ScanService.instance.isScanning.value && mounted) {
      _loadSources();
    }
  }

  Future<void> _loadSources() async {
    final sources = await ScanService.instance.getLibrarySources();
    if (mounted) {
      setState(() {
        _sources = sources;
      });
    }
  }

  Future<void> _showAddSourceDialog() async {
    final results = await showDialog<List<_AddSourceResult>>(
      context: context,
      builder: (context) => const _AddSourceDialog(),
    );
    if (results == null || results.isEmpty) return;

    bool anyAdded = false;
    for (final result in results) {
      final added = await ScanService.instance.addSource(
        result.url,
        result.name,
        sourceType: result.sourceType,
      );
      if (added) anyAdded = true;
    }
    if (anyAdded) await _loadSources();
  }

  Future<void> _removeSource(String sourceId) async {
    await ScanService.instance.removeSource(sourceId);
    await _loadSources();
  }

  Future<void> _showForceRescanDialog() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Force Rescan'),
        content: const Text(
          'This will clear your entire library and re-scan all configured music folders.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Rescan'),
          ),
        ],
      ),
    );
    if (confirmed == true && mounted) {
      ScanService.instance.forceRescan();
    }
  }

  double? _progressValue(ScanProgressSignal p) {
    const totalPhases = 6;
    final idx = _stageIndex(p.stage);
    if (idx < 0) return null;
    if (idx >= totalPhases) return 1.0;
    if (p.stage == 'scanning' && p.totalFiles > 0) {
      return (idx + p.filesScanned / p.totalFiles) / totalPhases;
    }
    return idx / totalPhases;
  }

  int _stageIndex(String stage) {
    const stages = [
      'enumerating',
      'comparing',
      'diffing',
      'scanning',
      'cleaning',
      'cover_update',
      'done',
    ];
    return stages.indexOf(stage);
  }

  String _stageLabel(ScanProgressSignal p) {
    final src = p.currentSource.isNotEmpty ? '${p.currentSource} — ' : '';
    switch (p.stage) {
      case 'enumerating':
        return '${src}Phase 1/6 — Enumerating files...';
      case 'comparing':
        return '${src}Phase 2/6 — Comparing with database...';
      case 'diffing':
        return '${src}Phase 3/6 — Computing differences...';
      case 'scanning':
        {
          final sub = p.totalFiles > 0
              ? ' (${p.filesScanned}/${p.totalFiles})'
              : '';
          return '${src}Phase 4/6 — Scanning files$sub...';
        }
      case 'cleaning':
        return '${src}Phase 5/6 — Cleaning removed files...';
      case 'cover_update':
        return '${src}Phase 6/6 — Updating album covers...';
      case 'done':
        return 'Scan complete';
      default:
        return p.stage;
    }
  }

  Widget _resultChip(Color color, IconData icon, String label, String hint) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          icon,
          size: AppTheme.iconSM * AppTheme.iconScale(context),
          color: color,
        ),
        SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
        Text(
          label,
          style: TextStyle(
            fontSize: AppTheme.textSM * AppTheme.textScale(context),
            fontWeight: FontWeight.w600,
            color: color,
          ),
        ),
        SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
        Text(
          hint,
          style: TextStyle(
            fontSize: AppTheme.textSM * AppTheme.textScale(context),
            color: color.withValues(alpha: 0.7),
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;

    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        const SectionHeader(
          title: 'Library',
          leading: Icon(Icons.library_music),
        ),
        if (_sources.isEmpty)
          Padding(
            padding: EdgeInsets.symmetric(
              vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
            ),
            child: Text(
              'No library sources configured. Add folders or remote libraries containing your music files.',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          )
        else
          ...List.generate(_sources.length, (i) {
            final source = _sources[i];
            final isLocal = source.sourceType == 'local';
            return Card(
              margin: EdgeInsets.symmetric(
                vertical: AppTheme.spaceXS * AppTheme.spaceScale(context),
              ),
              child: ListTile(
                leading: Icon(
                  isLocal ? Icons.folder_outlined : Icons.dns_outlined,
                  color: colorScheme.primary,
                ),
                title: Row(
                  children: [
                    Expanded(
                      child: Text(
                        source.name.isNotEmpty ? source.name : source.url,
                        style: textTheme.bodyMedium,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    SizedBox(
                      width: AppTheme.spaceSM * AppTheme.spaceScale(context),
                    ),
                    Container(
                      padding: EdgeInsets.symmetric(
                        horizontal:
                            AppTheme.spaceSM * AppTheme.spaceScale(context),
                        vertical:
                            AppTheme.spaceXS * AppTheme.spaceScale(context),
                      ),
                      decoration: BoxDecoration(
                        color: colorScheme.secondaryContainer,
                        borderRadius: BorderRadius.circular(
                          AppTheme.radiusSM * AppTheme.radiusScale(context),
                        ),
                      ),
                      child: Text(
                        source.sourceType,
                        style: textTheme.labelSmall?.copyWith(
                          color: colorScheme.onSecondaryContainer,
                        ),
                      ),
                    ),
                  ],
                ),
                subtitle: Text(
                  source.url,
                  style: textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
                trailing: IconButton(
                  icon: Icon(
                    Icons.remove_circle_outline,
                    color: colorScheme.error,
                  ),
                  onPressed: () => _removeSource(source.id),
                  tooltip: 'Remove source',
                ),
              ),
            );
          }),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        OutlinedButton.icon(
          onPressed: _showAddSourceDialog,
          icon: const Icon(Icons.add),
          label: const Text('Add Source'),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListenableBuilder(
          listenable: Listenable.merge([
            ScanService.instance.isScanning,
            ScanService.instance.progress,
          ]),
          builder: (context, _) {
            final s = ScanService.instance;
            final scanning = s.isScanning.value;
            final p = s.progress.value;
            return Column(
              children: [
                Row(
                  children: [
                    Expanded(
                      child: FilledButton.tonalIcon(
                        onPressed: scanning
                            ? null
                            : ScanService.instance.incrementalScan,
                        icon: const Icon(Icons.playlist_add_check_outlined),
                        label: const Text('Incremental Scan'),
                      ),
                    ),
                    SizedBox(
                      width: AppTheme.spaceSM * AppTheme.spaceScale(context),
                    ),
                    Expanded(
                      child: FilledButton.tonalIcon(
                        onPressed: scanning ? null : _showForceRescanDialog,
                        icon: const Icon(Icons.refresh),
                        label: const Text('Force Rescan'),
                      ),
                    ),
                  ],
                ),
                if (scanning && p != null) ...[
                  Padding(
                    padding: EdgeInsets.only(
                      top: AppTheme.spaceSM * AppTheme.spaceScale(context),
                    ),
                    child: LinearProgressIndicator(value: _progressValue(p)),
                  ),
                  Padding(
                    padding: EdgeInsets.only(
                      top: AppTheme.spaceXS * AppTheme.spaceScale(context),
                    ),
                    child: Text(
                      _stageLabel(p),
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  if (p.stage == 'scanning' && p.currentFile.isNotEmpty)
                    Padding(
                      padding: EdgeInsets.only(
                        top: AppTheme.spaceXS * AppTheme.spaceScale(context),
                      ),
                      child: Text(
                        p.currentFile,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant.withValues(
                            alpha: 0.6,
                          ),
                        ),
                        overflow: TextOverflow.ellipsis,
                        maxLines: 1,
                      ),
                    ),
                  if (p.stage == 'done') ...[
                    SizedBox(
                      height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                    ),
                    Row(
                      children: [
                        _resultChip(
                          colorScheme.primary,
                          Icons.library_music_outlined,
                          '${p.tracksFound}',
                          'Found',
                        ),
                        SizedBox(
                          width:
                              AppTheme.spaceSM * AppTheme.spaceScale(context),
                        ),
                        _resultChip(
                          colorScheme.tertiary,
                          Icons.add_circle_outline,
                          '${p.newTracks}',
                          'New',
                        ),
                        SizedBox(
                          width:
                              AppTheme.spaceSM * AppTheme.spaceScale(context),
                        ),
                        _resultChip(
                          colorScheme.error,
                          Icons.repeat,
                          '${p.duplicates}',
                          'Dups',
                        ),
                        SizedBox(
                          width:
                              AppTheme.spaceSM * AppTheme.spaceScale(context),
                        ),
                        _resultChip(
                          colorScheme.secondary,
                          Icons.delete_outline,
                          '${p.deleted}',
                          'Deleted',
                        ),
                      ],
                    ),
                  ],
                ],
              ],
            );
          },
        ),

        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        Text(
          'Tawai will scan local folders and remote libraries for music files.',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),

        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),

        // Metadata section
        const SectionHeader(title: 'Metadata', leading: Icon(Icons.tag)),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ValueListenableBuilder<String>(
          valueListenable: SettingsManager.namingPattern,
          builder: (context, value, _) {
            final isAdmin = SettingsManager.currentUser.value?.role == 'admin';
            return ListDropdown(
              title: 'Naming Format',
              subtitle: 'Pattern for organizing files.',
              valueListenable: SettingsManager.namingPattern,
              items: const [
                DropdownMenuItem(
                  value:
                      '{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}',
                  child: Text('Picard-style'),
                ),
                DropdownMenuItem(
                  value:
                      '{album}{album_disambiguation? (|)}/{album_artist?|_}{album?|_}{disc_prefix}{track_padded}_{title}',
                  child: Text('Album/DirPrefix_TrackNo_Title'),
                ),
                DropdownMenuItem(
                  value: '{artist}/{album}/{track_padded} - {title}',
                  child: Text('Artist/Album/## - Title'),
                ),
                DropdownMenuItem(
                  value: '{artist}/{album}/{track_padded} {title}',
                  child: Text('Artist/Album/## Title'),
                ),
                DropdownMenuItem(
                  value: '{artist} - {track_padded} - {title}',
                  child: Text('Artist - ## - Title'),
                ),
                DropdownMenuItem(
                  value: '{track_padded} - {title}',
                  child: Text('## - Title'),
                ),
                DropdownMenuItem(
                  value: '{artist} - {album}/{track_padded} - {title}',
                  child: Text('Artist - Album/## - Title'),
                ),
              ],
              editable: true,
              enabled: isAdmin,
              suffixWidgets: [
                IconButton(
                  icon: Icon(
                    Icons.help_outline,
                    size: AppTheme.iconMD * AppTheme.iconScale(context),
                  ),
                  tooltip: 'Naming Format Help',
                  onPressed: () => showNamingFormatHelpDialog(context),
                ),
              ],
              onChange: (v) {
                SettingsManager.namingPattern.value = v;
                SettingsManager.syncNamingPatternToRust(v);
              },
            );
          },
        ),
        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Prefer synced lyrics',
          subtitle:
              'When available, prefer synced (timed) lyrics over plain text',
          valueListenable: SettingsManager.lyricsPrefersync,
          onChanged: (v) => SettingsManager.saveUserSetting(
            SettingsManager.lyricsPrefersync,
            'lyrics_prefersync',
            v,
          ),
        ),
        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        Text(
          'The naming format is applied during file organization. '
          'Lyrics preference affects which lyrics format is fetched from providers.',
          style: textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),

        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),

        // Discovery section
        const SectionHeader(title: 'Discovery', leading: Icon(Icons.explore)),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListTextField(
          title: 'ListenBrainz Token',
          subtitle: 'Required for scrobbling and music discovery',
          valueListenable: SettingsManager.listenbrainzToken,
          isObscured: true,
          onConfirm: (value) => SettingsManager.saveUserSetting(
            SettingsManager.listenbrainzToken,
            'listenbrainz_token',
            value,
          ),
        ),
        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),

        // Recommendation sources as library sources
        const SectionHeader(
          title: 'Recommendation Sources',
          leading: Icon(Icons.explore),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        Text(
          'Select ListenBrainz recommendation types to include as library '
          'sources. Their tracks will appear in the library and can be '
          'played via yt-dlp.',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ValueListenableBuilder<String>(
          valueListenable: SettingsManager.includedRecommendations,
          builder: (context, raw, _) {
            final selected = raw.split(',').where((s) => s.isNotEmpty).toSet();
            return Column(
              children: [
                for (final src in RecommendationSource.all)
                  CheckboxListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    title: Text(src.displayName, style: textTheme.bodyMedium),
                    value: selected.contains(src.key),
                    onChanged: (checked) {
                      final updated = Set<String>.from(selected);
                      if (checked == true) {
                        updated.add(src.key);
                      } else {
                        updated.remove(src.key);
                      }
                      SettingsManager.saveUserSetting(
                        SettingsManager.includedRecommendations,
                        'included_recommendations',
                        updated.join(','),
                      );
                    },
                  ),
              ],
            );
          },
        ),

        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        Text(
          'Link your ListenBrainz account to enable scrobbling '
          'and discover new music based on your listening habits.',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

class _AddSourceResult {
  final String url;
  final String name;
  final String sourceType;
  _AddSourceResult({
    required this.url,
    required this.name,
    required this.sourceType,
  });
}

class _AddSourceDialog extends StatefulWidget {
  const _AddSourceDialog();

  @override
  State<_AddSourceDialog> createState() => _AddSourceDialogState();
}

class _AddSourceDialogState extends State<_AddSourceDialog> {
  String _sourceType = 'local';
  String _localPath = '';

  late final TextEditingController _urlCtrl;
  late final TextEditingController _usernameCtrl;
  late final TextEditingController _passwordCtrl;
  late final TextEditingController _nameCtrl;

  List<JellyfinLibraryInfo> _jellyfinLibraries = [];
  Set<String> _selectedLibraryIds = {};
  bool _testing = false;
  String? _testError;

  @override
  void initState() {
    super.initState();
    _urlCtrl = TextEditingController();
    _usernameCtrl = TextEditingController();
    _passwordCtrl = TextEditingController();
    _nameCtrl = TextEditingController();
    _urlCtrl.addListener(_onFieldChanged);
    _usernameCtrl.addListener(_onFieldChanged);
    _passwordCtrl.addListener(_onFieldChanged);
  }

  void _onFieldChanged() => setState(() {});

  @override
  void dispose() {
    _urlCtrl.removeListener(_onFieldChanged);
    _usernameCtrl.removeListener(_onFieldChanged);
    _passwordCtrl.removeListener(_onFieldChanged);
    _urlCtrl.dispose();
    _usernameCtrl.dispose();
    _passwordCtrl.dispose();
    _nameCtrl.dispose();
    super.dispose();
  }

  Future<void> _pickFolder() async {
    final path = await pickFolder(
      context,
      initialPath: _localPath.isNotEmpty ? _localPath : null,
    );
    if (path != null) {
      setState(() {
        _localPath = path;
        if (_nameCtrl.text.isEmpty) _nameCtrl.text = path.split('/').last;
      });
    }
  }

  Future<void> _testConnection() async {
    final scheme = _urlCtrl.text.startsWith('https') ? 'https' : 'http';
    final host = _urlCtrl.text.replaceAll(RegExp(r'^https?://'), '');
    final url = '$scheme://${_usernameCtrl.text}:${_passwordCtrl.text}@$host';

    setState(() {
      _testing = true;
      _testError = null;
      _jellyfinLibraries = [];
      _selectedLibraryIds = {};
    });

    try {
      final libraries = await BridgeService.instance.testJellyfinSource(url);
      setState(() {
        _testing = false;
        _jellyfinLibraries = libraries;
        _selectedLibraryIds = libraries.map((l) => l.id).toSet();
      });
    } catch (e) {
      setState(() {
        _testing = false;
        _testError = e.toString();
      });
    }
  }

  void _submit() {
    if (_sourceType == 'local' && _localPath.isEmpty) return;

    String baseUrl;
    String defaultName;
    if (_sourceType == 'local') {
      baseUrl = _localPath;
      defaultName = _localPath.split('/').last;
    } else {
      final scheme = _urlCtrl.text.startsWith('https') ? 'https' : 'http';
      final host = _urlCtrl.text.replaceAll(RegExp(r'^https?://'), '');
      baseUrl = '$scheme://${_usernameCtrl.text}:${_passwordCtrl.text}@$host';
      defaultName = host.split(':').first;
    }

    final results = <_AddSourceResult>[];
    final name = _nameCtrl.text;

    if (_sourceType == 'jellyfin' && _jellyfinLibraries.isNotEmpty) {
      for (final lib in _jellyfinLibraries) {
        if (!_selectedLibraryIds.contains(lib.id)) continue;
        final url = '$baseUrl?libraryId=${lib.id}';
        final srcName = name.isNotEmpty
            ? '$name - ${lib.name}'
            : '$defaultName - ${lib.name}';
        results.add(
          _AddSourceResult(url: url, name: srcName, sourceType: _sourceType),
        );
      }
    } else {
      results.add(
        _AddSourceResult(
          url: baseUrl,
          name: name.isNotEmpty ? name : defaultName,
          sourceType: _sourceType,
        ),
      );
    }

    if (mounted) Navigator.of(context).pop(results);
  }

  bool get _fieldsFilled =>
      _urlCtrl.text.isNotEmpty &&
      _usernameCtrl.text.isNotEmpty &&
      _passwordCtrl.text.isNotEmpty;

  int _addCount() {
    if (_sourceType == 'local') return 1;
    if (_jellyfinLibraries.isNotEmpty) return _selectedLibraryIds.length;
    return 1;
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;

    final canSubmit = _sourceType == 'local'
        ? _localPath.isNotEmpty
        : _fieldsFilled;

    return AlertDialog(
      title: const Text('Add Library Source'),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Source Type', style: textTheme.labelMedium),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(
                  value: 'local',
                  label: Text('Local Folder'),
                  icon: Icon(Icons.folder_outlined),
                ),
                ButtonSegment(
                  value: 'jellyfin',
                  label: Text('Jellyfin'),
                  icon: Icon(Icons.dns_outlined),
                ),
              ],
              selected: {_sourceType},
              onSelectionChanged: (v) => setState(() {
                _sourceType = v.first;
                _testError = null;
                _jellyfinLibraries = [];
                _selectedLibraryIds = {};
              }),
            ),
            SizedBox(
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),

            if (_sourceType == 'local') ...[
              Row(
                children: [
                  Expanded(
                    child: Text(
                      _localPath.isEmpty ? 'No folder selected' : _localPath,
                      style: textTheme.bodySmall,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  SizedBox(
                    width: AppTheme.spaceSM * AppTheme.spaceScale(context),
                  ),
                  OutlinedButton(
                    onPressed: _pickFolder,
                    child: const Text('Browse'),
                  ),
                ],
              ),
            ] else ...[
              TextField(
                controller: _urlCtrl,
                decoration: const InputDecoration(
                  labelText: 'Server URL',
                  hintText: 'jellyfin.local:8096',
                  helperText: 'Hostname and port of your Jellyfin server',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onChanged: (_) => _testError = null,
              ),
              SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
              TextField(
                controller: _usernameCtrl,
                decoration: const InputDecoration(
                  labelText: 'Username',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onChanged: (_) => _testError = null,
              ),
              SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
              TextField(
                controller: _passwordCtrl,
                decoration: const InputDecoration(
                  labelText: 'Password',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                obscureText: true,
                onChanged: (_) => _testError = null,
              ),
              SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: _fieldsFilled ? _testConnection : null,
                  icon: _testing
                      ? SizedBox(
                          width:
                              AppTheme.spaceSM *
                              2 *
                              AppTheme.spaceScale(context),
                          height:
                              AppTheme.spaceSM *
                              2 *
                              AppTheme.spaceScale(context),
                          child: const CircularProgressIndicator(
                            strokeWidth: 2,
                          ),
                        )
                      : const Icon(Icons.wifi_find),
                  label: Text(_testing ? 'Testing...' : 'Test Connection'),
                ),
              ),
              if (_testError != null) ...[
                SizedBox(
                  height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                Text(
                  _testError!,
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.error,
                  ),
                ),
              ],
              if (_jellyfinLibraries.isNotEmpty) ...[
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
                Text('Music Libraries', style: textTheme.labelMedium),
                SizedBox(
                  height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                ),
                ..._jellyfinLibraries.map(
                  (lib) => CheckboxListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    title: Text(lib.name, style: textTheme.bodySmall),
                    value: _selectedLibraryIds.contains(lib.id),
                    onChanged: (checked) {
                      setState(() {
                        if (checked == true) {
                          _selectedLibraryIds.add(lib.id);
                        } else {
                          _selectedLibraryIds.remove(lib.id);
                        }
                      });
                    },
                  ),
                ),
              ],
            ],

            SizedBox(
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(
                labelText: 'Source Name',
                hintText: 'My Music',
                border: OutlineInputBorder(),
                isDense: true,
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
          onPressed: canSubmit ? _submit : null,
          child: Text('Add${_addCount() > 1 ? ' (${_addCount()})' : ''}'),
        ),
      ],
    );
  }
}
