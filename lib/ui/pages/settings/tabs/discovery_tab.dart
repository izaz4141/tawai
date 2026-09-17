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
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/models/recommendation_source.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';

class SettingsDiscoveryTab extends StatefulWidget {
  const SettingsDiscoveryTab({super.key});

  @override
  State<SettingsDiscoveryTab> createState() => _SettingsDiscoveryTabState();
}

String _redactUrl(String url) {
  if (url.startsWith('tawai://')) {
    final body = url.substring('tawai://'.length);
    final hostPort = body.contains('@') ? body.split('@').first : body;
    final sourceId = RegExp(r'source_id=([^&]*)').firstMatch(body)?.group(1);
    return hostPort + (sourceId == null || sourceId.isEmpty ? '' : '?source_id=$sourceId');
  }
  final at = url.lastIndexOf('@');
  return at >= 0 ? url.substring(at + 1) : url;
}

String _stripScheme(String raw) {
  final t = raw.trim();
  final noScheme = t.replaceAll(RegExp(r'^https?://', caseSensitive: false), '');
  return noScheme.replaceAll(RegExp(r'/+$'), '');
}

String _schemeOfRaw(String raw) =>
    RegExp(r'^https://', caseSensitive: false).hasMatch(raw.trim())
        ? 'https'
        : 'http';

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
        result.urls,
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

  Future<void> _syncRecommendations(String includedKeys) async {
    await BridgeService.instance.syncRecs(includedKeys: includedKeys);
    if (mounted) await _loadSources();
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
            final icon = isLocal
                ? Icons.folder_outlined
                : source.sourceType == 'tawai'
                    ? Icons.cloud_outlined
                    : Icons.dns_outlined;
            return Card(
              margin: EdgeInsets.symmetric(
                vertical: AppTheme.spaceXS * AppTheme.spaceScale(context),
              ),
              child: ListTile(
                leading: Icon(icon, color: colorScheme.primary),
                title: Row(
                  children: [
                    Expanded(
                      child: Text(
                        source.name.isNotEmpty ? source.name : (source.urls.isNotEmpty ? _redactUrl(source.urls.first) : ''),
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
                  source.urls.map(_redactUrl).join(' → '),
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
                      unawaited(_syncRecommendations(updated.join(',')));
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
  final List<String> urls;
  final String name;
  final String sourceType;
  _AddSourceResult({
    required this.urls,
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
  late final TextEditingController _tawaiServersCtrl;
  late final TextEditingController _tawaiApiKeyCtrl;

  List<JellyfinLibraryInfo> _jellyfinLibraries = [];
  Set<String> _selectedLibraryIds = {};
  bool _testing = false;
  String? _testError;
  bool _tawaiSourcesEmpty = false;

  @override
  void initState() {
    super.initState();
    _urlCtrl = TextEditingController();
    _usernameCtrl = TextEditingController();
    _passwordCtrl = TextEditingController();
    _nameCtrl = TextEditingController();
    _tawaiServersCtrl = TextEditingController();
    _tawaiApiKeyCtrl = TextEditingController();
    _urlCtrl.addListener(_onFieldChanged);
    _usernameCtrl.addListener(_onFieldChanged);
    _passwordCtrl.addListener(_onFieldChanged);
    _tawaiServersCtrl.addListener(_onFieldChanged);
    _tawaiApiKeyCtrl.addListener(_onFieldChanged);
  }

  void _onFieldChanged() => setState(() {});

  @override
  void dispose() {
    _urlCtrl.removeListener(_onFieldChanged);
    _usernameCtrl.removeListener(_onFieldChanged);
    _passwordCtrl.removeListener(_onFieldChanged);
    _tawaiServersCtrl.removeListener(_onFieldChanged);
    _tawaiApiKeyCtrl.removeListener(_onFieldChanged);
    _urlCtrl.dispose();
    _usernameCtrl.dispose();
    _passwordCtrl.dispose();
    _nameCtrl.dispose();
    _tawaiServersCtrl.dispose();
    _tawaiApiKeyCtrl.dispose();
    super.dispose();
  }

  String _schemeOf(String raw) => _schemeOfRaw(raw);

  String _tawaiUrl(String raw, String key, String sid) {
    final clean = _stripScheme(raw);
    final sidT = sid.trim();
    final query = sidT.isEmpty
        ? 'scheme=${_schemeOf(raw)}'
        : 'source_id=$sidT&scheme=${_schemeOf(raw)}';
    return 'tawai://$clean@${key.trim()}?$query';
  }

  Future<void> _pickFolder() async {
    final path = await IOServiceFactory.create().getDirectoryPath(
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
    final urls = <String>[];

    if (_sourceType == 'tawai') {
      final servers = _tawaiServersCtrl.text
          .split(',')
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      if (servers.isEmpty) {
        const msg = 'No server address provided';
        setState(() {
          _testing = false;
          _testError = msg;
        });
        AppSnackBar.show(context, msg, type: SnackType.error);
        return;
      }
      if (_tawaiApiKeyCtrl.text.trim().isEmpty) {
        const msg = 'No API key provided';
        setState(() {
          _testing = false;
          _testError = msg;
        });
        AppSnackBar.show(context, msg, type: SnackType.error);
        return;
      }
      for (final server in servers) {
        urls.add(_tawaiUrl(server, _tawaiApiKeyCtrl.text, ''));
      }
    } else {
      final hosts = _urlCtrl.text
          .split(',')
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      if (hosts.isEmpty) {
        const msg = 'No server URL provided';
        setState(() {
          _testing = false;
          _testError = msg;
        });
        AppSnackBar.show(context, msg, type: SnackType.error);
        return;
      }
      for (final host in hosts) {
        final clean = _stripScheme(host);
        urls.add(
          '${_schemeOf(host)}://${_usernameCtrl.text}:${_passwordCtrl.text}@$clean',
        );
      }
    }

    setState(() {
      _testing = true;
      _testError = null;
      _jellyfinLibraries = [];
      _selectedLibraryIds = {};
      _tawaiSourcesEmpty = false;
    });

    try {
      final res = await BridgeService.instance.testSource(_sourceType, urls);
      if (!mounted) return;

      final reachable = res.results.where((r) => r.reachable).toList();
      final failed = res.results.where((r) => !r.reachable).toList();

      setState(() {
        _testing = false;
        if (res.libraries.isNotEmpty) {
          _jellyfinLibraries = res.libraries;
          _selectedLibraryIds = res.libraries.map((l) => l.id).toSet();
        }
      });

      if (_sourceType == 'tawai' &&
          reachable.isNotEmpty &&
          res.libraries.isEmpty) {
        setState(() => _tawaiSourcesEmpty = true);
        AppSnackBar.show(
          context,
          'Connected, but no library sources are accessible with this '
          'API key.',
          type: SnackType.info,
        );
      }

      if (failed.isNotEmpty) {
        final failedLabels = failed.map((r) {
          final label = _redactUrl(r.url);
          final detail = r.error;
          return (detail == null || detail.isEmpty)
              ? label
              : '$label — $detail';
        }).toList();
        if (reachable.isEmpty) {
          setState(() {
            _testError = 'All servers unreachable: ${failedLabels.join(', ')}';
          });
          AppSnackBar.show(
            context,
            'Unable to reach any server:\n${failedLabels.join('\n')}',
            type: SnackType.error,
          );
        } else {
          setState(() {
            _testError =
                '${failed.length} of ${urls.length} server(s) unreachable: '
                '${failedLabels.join(', ')}. Reachable servers act as fallbacks.';
          });
          AppSnackBar.show(
            context,
            '${failed.length} of ${urls.length} server(s) unreachable:\n'
            '${failedLabels.join('\n')}\nReachable servers act as fallbacks.',
            type: SnackType.error,
          );
        }
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _testing = false;
        _testError = e.toString();
      });
      AppSnackBar.show(context, e.toString(), type: SnackType.error);
    }
  }

  void _submit() {
    if (_sourceType == 'local' && _localPath.isEmpty) return;

    final results = <_AddSourceResult>[];
    final name = _nameCtrl.text;

    if (_sourceType == 'local') {
      results.add(
        _AddSourceResult(
          urls: [_localPath],
          name: name.isNotEmpty ? name : _localPath.split('/').last,
          sourceType: _sourceType,
        ),
      );
      if (mounted) Navigator.of(context).pop(results);
      return;
    }

    if (_sourceType == 'tawai') {
      final servers = _tawaiServersCtrl.text
          .split(',')
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      if (servers.isEmpty) {
        AppSnackBar.show(
          context,
          'Enter at least one server address',
          type: SnackType.error,
        );
        return;
      }
      final selected = _jellyfinLibraries
          .where((lib) => _selectedLibraryIds.contains(lib.id))
          .toList();
      if (selected.isEmpty) {
        AppSnackBar.show(
          context,
          'Select at least one library source from the test results',
          type: SnackType.error,
        );
        return;
      }
      final hostName = _stripScheme(servers.first).split(':').first;
      for (final lib in selected) {
        final urls = <String>[
          _localPath,
          for (final server in servers)
            _tawaiUrl(server, _tawaiApiKeyCtrl.text, lib.id),
        ];
        final srcName = name.isNotEmpty
            ? '$name - ${lib.name}'
            : '$hostName - ${lib.name}';
        results.add(
          _AddSourceResult(urls: urls, name: srcName, sourceType: _sourceType),
        );
      }
      if (mounted) Navigator.of(context).pop(results);
      return;
    }

    final hosts = _urlCtrl.text
        .split(',')
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    if (hosts.isEmpty) {
      AppSnackBar.show(
        context,
        'Enter at least one server address',
        type: SnackType.error,
      );
      return;
    }
    final baseUrls = [
      for (final host in hosts)
        '${_schemeOf(host)}://${_usernameCtrl.text}:${_passwordCtrl.text}@${_stripScheme(host)}',
    ];
    final defaultName = _stripScheme(hosts.first).split(':').first;

    if (_jellyfinLibraries.isNotEmpty) {
      for (final lib in _jellyfinLibraries) {
        if (!_selectedLibraryIds.contains(lib.id)) continue;
        final urls = [
          for (final baseUrl in baseUrls) '$baseUrl?libraryId=${lib.id}',
        ];
        final srcName = name.isNotEmpty
            ? '$name - ${lib.name}'
            : '$defaultName - ${lib.name}';
        results.add(
          _AddSourceResult(urls: urls, name: srcName, sourceType: _sourceType),
        );
      }
    } else {
      results.add(
        _AddSourceResult(
          urls: baseUrls,
          name: name.isNotEmpty ? name : defaultName,
          sourceType: _sourceType,
        ),
      );
    }

    if (mounted) Navigator.of(context).pop(results);
  }

bool get _fieldsFilled =>
      _urlCtrl.text.trim().isNotEmpty &&
      _usernameCtrl.text.isNotEmpty &&
      _passwordCtrl.text.isNotEmpty;

bool get _tawaiFieldsFilled =>
      _tawaiServersCtrl.text.trim().isNotEmpty &&
      _tawaiApiKeyCtrl.text.trim().isNotEmpty;

int _addCount() {
  if (_sourceType == 'local') return 1;
  if (_sourceType == 'tawai' || _jellyfinLibraries.isNotEmpty) {
    return _selectedLibraryIds.length;
  }
  return 1;
}

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;

    final canSubmit = _sourceType == 'local'
        ? _localPath.isNotEmpty
        : _sourceType == 'tawai'
            ? _localPath.isNotEmpty &&
                _tawaiFieldsFilled &&
                _selectedLibraryIds.isNotEmpty
            : _fieldsFilled;

    final canTest = _sourceType == 'tawai' ? _tawaiFieldsFilled : _fieldsFilled;

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
                ButtonSegment(
                  value: 'tawai',
                  label: Text('Tawai'),
                  icon: Icon(Icons.cloud_outlined),
                ),
              ],
              selected: {_sourceType},
              onSelectionChanged: (v) => setState(() {
                _sourceType = v.first;
                _testError = null;
                _jellyfinLibraries = [];
                _selectedLibraryIds = {};
                _tawaiSourcesEmpty = false;
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
              if (_sourceType == 'jellyfin') ...[
                TextField(
                  controller: _urlCtrl,
                  decoration: const InputDecoration(
                    labelText: 'Server URL',
                    hintText: 'https://jellyfin.local:8096, http://192.168.1.5:8096',
                    helperText:
                        'Multiple server URLs separated by commas. First is '
                        'preferred (e.g. home network), the rest are used as '
                        'fallbacks. Include http:// or https://; plain hosts '
                        'default to http.',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                  onChanged: (_) => _testError = null,
                ),
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
                TextField(
                  controller: _usernameCtrl,
                  decoration: const InputDecoration(
                    labelText: 'Username',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                  onChanged: (_) => _testError = null,
                ),
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
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
              ] else ...[
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        _localPath.isEmpty
                            ? 'No backup folder selected'
                            : _localPath,
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
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
                TextField(
                  controller: _tawaiServersCtrl,
                  decoration: const InputDecoration(
                    labelText: 'Server Addresses',
                    hintText:
                        'https://ac.ex.com:443, https://ac.ss.com:443, http://127.0.0.1:8181',
                    helperText:
                        'Multiple server URLs separated by commas. First is '
                        'preferred (e.g. home network), the rest are used as '
                        'fallbacks (e.g. remote network). Include http:// or '
                        'https://; plain hosts default to http. Test the '
                        'connection to list the library sources to add.',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                  onChanged: (_) => _testError = null,
                ),
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
                TextField(
                  controller: _tawaiApiKeyCtrl,
                  decoration: const InputDecoration(
                    labelText: 'API Key',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                  obscureText: true,
                  onChanged: (_) => _testError = null,
                ),
              ],
              SizedBox(
                height: AppTheme.spaceMD * AppTheme.spaceScale(context),
              ),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: canTest ? _testConnection : null,
                  icon: _testing
                      ? SizedBox(
                          width: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
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
              if (_sourceType == 'tawai' && _tawaiSourcesEmpty) ...[
                SizedBox(
                  height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                Text(
                  'Connected, but no library sources are accessible with this '
                  'API key on the selected server.',
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              if (_jellyfinLibraries.isNotEmpty) ...[
                SizedBox(
                  height: AppTheme.spaceMD * AppTheme.spaceScale(context),
                ),
                Text(
                  _sourceType == 'tawai'
                      ? 'Library Sources'
                      : 'Music Libraries',
                  style: textTheme.labelMedium,
                ),
                SizedBox(
                  height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                ),
                ..._jellyfinLibraries.map(
                  (lib) => CheckboxListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    title: Text(
                      lib.name.isNotEmpty ? lib.name : lib.id,
                      style: textTheme.bodySmall,
                    ),
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
