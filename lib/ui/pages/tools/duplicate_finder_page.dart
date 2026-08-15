import 'dart:async';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/track_list_tile.dart';
import 'package:tawai/ui/widgets/mini_player.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/logger.dart';

class DuplicateFinderPage extends StatefulWidget {
  const DuplicateFinderPage({super.key});

  @override
  State<DuplicateFinderPage> createState() => _DuplicateFinderPageState();
}

class _DuplicateFinderPageState extends State<DuplicateFinderPage> {
  bool _checkFingerprint = true;
  bool _checkMbid = true;
  bool _checkFileSizeDuration = true;
  bool _checkTitleArtist = false;
  bool _scanning = false;
  bool _resolvingTracks = false;
  List<DuplicateGroup> _groups = [];
  Map<String, TrackInfo> _trackInfos = {};
  bool _hasScanned = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    BridgeService.libraryChanged.addListener(_onLibraryChanged);
  }

  @override
  void dispose() {
    BridgeService.libraryChanged.removeListener(_onLibraryChanged);
    super.dispose();
  }

  void _onLibraryChanged() {
    if (mounted) unawaited(_resolveTracks());
  }

  Future<void> _scan() async {
    setState(() {
      _scanning = true;
      _error = null;
      _groups = [];
    });
    try {
      final response = await BridgeService.instance.findDuplicates(
        checkFingerprint: _checkFingerprint,
        checkMbid: _checkMbid,
        checkFileSizeDuration: _checkFileSizeDuration,
        checkTitleArtist: _checkTitleArtist,
      );
      if (!mounted) return;
      if (response != null) {
        setState(() {
          _groups = response.groups;
          _hasScanned = true;
          _scanning = false;
          _trackInfos = {};
        });
        unawaited(_resolveTracks());
      } else {
        setState(() {
          _error = 'Failed to scan for duplicates';
          _scanning = false;
        });
      }
    } catch (e) {
      log('duplicate_finder: scan error: $e', isError: true);
      if (mounted) {
        setState(() {
          _error = e.toString();
          _scanning = false;
        });
      }
    }
  }

  Future<void> _resolveTracks() async {
    final ids = <String>{
      for (final group in _groups)
        for (final entry in group.tracks) entry.trackId,
    };
    if (ids.isEmpty) {
      if (mounted) {
        setState(() {
          _trackInfos = {};
          _resolvingTracks = false;
        });
      }
      return;
    }
    if (mounted) setState(() => _resolvingTracks = true);
    final results = await Future.wait(
      ids.map(
        (id) async => (id, await BridgeService.instance.getTrackInfo(id)),
      ),
    );
    final resolved = <String, TrackInfo>{};
    for (final (id, info) in results) {
      if (info != null) resolved[id] = info;
    }
    final newGroups = <DuplicateGroup>[
      for (final group in _groups)
        DuplicateGroup(
          method: group.method,
          key: group.key,
          confidence: group.confidence,
          tracks: [
            for (final entry in group.tracks)
              if (resolved.containsKey(entry.trackId)) entry,
          ],
        ),
    ].where((group) => group.tracks.isNotEmpty).toList();
    if (!mounted) return;
    setState(() {
      _trackInfos = resolved;
      _groups = newGroups;
      _resolvingTracks = false;
    });
  }

  List<MapEntry<String, List<DuplicateGroup>>> _methodBuckets() {
    const order = ['file_size_duration', 'mbid', 'fingerprint', 'title_artist'];
    final byMethod = <String, List<DuplicateGroup>>{};
    for (final group in _groups) {
      byMethod.putIfAbsent(group.method, () => []).add(group);
    }
    final buckets = <MapEntry<String, List<DuplicateGroup>>>[];
    for (final method in order) {
      final groups = byMethod[method];
      if (groups != null) buckets.add(MapEntry(method, groups));
    }
    for (final entry in byMethod.entries) {
      if (!order.contains(entry.key)) buckets.add(entry);
    }
    return buckets;
  }

  IconData _methodIcon(String method) {
    switch (method) {
      case 'file_size_duration':
        return Icons.storage;
      case 'mbid':
        return Icons.music_note;
      case 'fingerprint':
        return Icons.fingerprint;
      case 'title_artist':
        return Icons.title;
      default:
        return Icons.help_outline;
    }
  }

  String _methodLabel(String method) {
    switch (method) {
      case 'file_size_duration':
        return 'File Size + Duration';
      case 'mbid':
        return 'MusicBrainz ID';
      case 'fingerprint':
        return 'Audio Fingerprint';
      case 'title_artist':
        return 'Title + Artist';
      default:
        return method;
    }
  }

  Color _confidenceColor(double confidence) {
    if (confidence >= 0.95) return Colors.green;
    if (confidence >= 0.8) return Colors.orange;
    return Colors.red;
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(
          'Duplicate Finder',
          style: isDesktop ? textTheme.titleLarge : null,
        ),
      ),
      body: Stack(
        children: [
          Column(
            children: [
              _buildOptionsPanel(colors, textTheme, isDesktop),
              const Divider(height: 1),
              if (_scanning)
                const Expanded(
                  child: Center(child: CircularProgressIndicator()),
                )
              else if (_error != null)
                Expanded(
                  child: Center(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Text(
                        _error!,
                        style: textTheme.bodyMedium?.copyWith(
                          color: colors.error,
                        ),
                      ),
                    ),
                  ),
                )
              else if (!_hasScanned)
                Expanded(
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.copy_all,
                          size: AppTheme.iconXXL * AppTheme.iconScale(context),
                          color: colors.onSurfaceVariant,
                        ),
                        SizedBox(
                          height:
                              AppTheme.spaceSM *
                              2 *
                              AppTheme.spaceScale(context),
                        ),
                        Text(
                          'Configure options above and tap Scan',
                          style: textTheme.bodyLarge?.copyWith(
                            color: colors.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                )
              else if (_groups.isEmpty)
                Expanded(
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.check_circle,
                          size: AppTheme.iconXXL * AppTheme.iconScale(context),
                          color: colors.primary,
                        ),
                        SizedBox(
                          height:
                              AppTheme.spaceSM *
                              2 *
                              AppTheme.spaceScale(context),
                        ),
                        Text(
                          'No duplicates found!',
                          style: textTheme.titleMedium,
                        ),
                      ],
                    ),
                  ),
                )
              else
                Expanded(
                  child: Column(
                    children: [
                      if (_resolvingTracks)
                        const LinearProgressIndicator(minHeight: 2),
                      Expanded(
                        child: ListView.builder(
                          padding: EdgeInsets.all(
                            AppTheme.spaceSM * AppTheme.spaceScale(context),
                          ),
                          itemCount: _methodBuckets().length + 1,
                          itemBuilder: (context, index) {
                            if (index == _methodBuckets().length) {
                              return const MiniPlayerSpacer();
                            }
                            return _buildMethodTile(
                              _methodBuckets()[index],
                              colors,
                              textTheme,
                            );
                          },
                        ),
                      ),
                    ],
                  ),
                ),
            ],
          ),
          const Positioned(left: 0, right: 0, bottom: 0, child: MiniPlayer()),
        ],
      ),
    );
  }

  Widget _buildOptionsPanel(
    ColorScheme colors,
    TextTheme textTheme,
    bool isDesktop,
  ) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
        AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
        AppTheme.spaceSM * AppTheme.spaceScale(context),
        AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
        AppTheme.spaceSM * AppTheme.spaceScale(context),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Expanded(
                child: Wrap(
                  spacing: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                  runSpacing: AppTheme.spaceXS * AppTheme.spaceScale(context),
                  children: [
                    FilterChip(
                      label: const Text('File Size + Duration'),
                      selected: _checkFileSizeDuration,
                      onSelected: (v) =>
                          setState(() => _checkFileSizeDuration = v),
                    ),
                    FilterChip(
                      label: const Text('Audio Fingerprint'),
                      selected: _checkFingerprint,
                      onSelected: (v) => setState(() => _checkFingerprint = v),
                    ),
                    FilterChip(
                      label: const Text('MusicBrainz ID'),
                      selected: _checkMbid,
                      onSelected: (v) => setState(() => _checkMbid = v),
                    ),
                    FilterChip(
                      label: const Text('Title + Artist'),
                      selected: _checkTitleArtist,
                      onSelected: (v) => setState(() => _checkTitleArtist = v),
                    ),
                  ],
                ),
              ),
              FilledButton.icon(
                onPressed: _scanning ? null : _scan,
                icon: _scanning
                    ? SizedBox(
                        width:
                            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        height:
                            AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        child: const CircularProgressIndicator(strokeWidth: 2),
                      )
                    : Icon(
                        Icons.search,
                        size: AppTheme.iconSM * AppTheme.iconScale(context),
                      ),
                label: Text(_scanning ? 'Scanning...' : 'Scan'),
              ),
            ],
          ),
          if (_hasScanned)
            Padding(
              padding: EdgeInsets.only(
                top: AppTheme.spaceXS * AppTheme.spaceScale(context),
              ),
              child: Text(
                '${_groups.fold<int>(0, (s, g) => s + g.tracks.length)} duplicates in ${_groups.length} groups',
                style: textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildMethodTile(
    MapEntry<String, List<DuplicateGroup>> bucket,
    ColorScheme colors,
    TextTheme textTheme,
  ) {
    final groups = bucket.value;
    final count = groups.fold<int>(
      0,
      (sum, group) => sum + group.tracks.length,
    );
    return Card(
      margin: EdgeInsets.only(
        bottom: AppTheme.spaceSM * AppTheme.spaceScale(context),
      ),
      child: ExpansionTile(
        initiallyExpanded: bucket.key == _methodBuckets().first.key,
        leading: Icon(_methodIcon(bucket.key), color: colors.primary),
        title: Text(_methodLabel(bucket.key), style: textTheme.bodyLarge),
        subtitle: Text(
          '$count duplicates',
          style: textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
        ),
        children: [
          for (final group in groups) ...[
            _buildGroupCaption(group, colors, textTheme),
            for (final entry in group.tracks)
              _buildTrackRow(entry, colors, textTheme),
          ],
        ],
      ),
    );
  }

  Widget _buildGroupCaption(
    DuplicateGroup group,
    ColorScheme colors,
    TextTheme textTheme,
  ) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
        AppTheme.spaceMD * AppTheme.spaceScale(context),
        AppTheme.spaceSM * AppTheme.spaceScale(context),
        AppTheme.spaceMD * AppTheme.spaceScale(context),
        AppTheme.spaceXS * AppTheme.spaceScale(context),
      ),
      child: Row(
        children: [
          Icon(
            Icons.circle,
            size: AppTheme.iconXS * AppTheme.iconScale(context),
            color: _confidenceColor(group.confidence),
          ),
          SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
          Expanded(
            child: Text(
              '${(group.confidence * 100).toInt()}% confidence · ${group.tracks.length} tracks',
              style: textTheme.labelSmall?.copyWith(
                color: colors.onSurfaceVariant,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTrackRow(
    DuplicateTrackEntry entry,
    ColorScheme colors,
    TextTheme textTheme,
  ) {
    final info = _trackInfos[entry.trackId];
    if (info == null) {
      return ListTile(
        dense: true,
        leading: SizedBox(
          width: AppTheme.spaceMD * AppTheme.spaceScale(context),
          height: AppTheme.spaceMD * AppTheme.spaceScale(context),
          child: Center(
            child: SizedBox(
              width: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              height: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              child: const CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
        ),
        title: Text(entry.title, maxLines: 1, overflow: TextOverflow.ellipsis),
      );
    }
    return TrackListTile(track: info);
  }
}
