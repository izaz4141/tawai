import 'dart:async';
import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';

Future<void> showNamingFormatHelpDialog(BuildContext context) {
  return showDialog(
    context: context,
    builder: (_) => const _NamingFormatDialog(),
  );
}

class _NamingFormatDialog extends StatefulWidget {
  const _NamingFormatDialog();

  @override
  State<_NamingFormatDialog> createState() => _NamingFormatDialogState();
}

class _NamingFormatDialogState extends State<_NamingFormatDialog> {
  List<TrackInfo> _allTracks = [];
  List<TrackInfo> _filteredTracks = [];
  TrackInfo? _selectedTrack;
  String _previewResult = '';
  bool _loading = true;
  bool _previewLoading = false;
  final TextEditingController _searchCtrl = TextEditingController();
  Timer? _debounce;
  bool _showTrackList = false;
  late ValueNotifier<String> _patternNotifier;
  late TextEditingController _patternCtrl;

  @override
  void initState() {
    super.initState();
    _patternNotifier = ValueNotifier(SettingsManager.namingPattern.value);
    _patternCtrl = TextEditingController(text: _patternNotifier.value);
    _patternNotifier.addListener(_updatePreview);
    _loadTracks();
  }

  @override
  void dispose() {
    _searchCtrl.dispose();
    _debounce?.cancel();
    _patternCtrl.dispose();
    _patternNotifier.removeListener(_updatePreview);
    _patternNotifier.dispose();
    super.dispose();
  }

  Future<void> _loadTracks() async {
    try {
      final tracks = await BridgeService.instance.getTracks();
      if (!mounted) return;
      setState(() {
        _allTracks = tracks;
        _filteredTracks = tracks;
        _loading = false;
        if (tracks.isNotEmpty) {
          _selectedTrack = tracks.first;
          _updatePreview();
        }
      });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _onSearchChanged(String query) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 200), () {
      if (!mounted) return;
      setState(() {
        _showTrackList = true;
        _filteredTracks = _allTracks
            .where((t) => t.title.toLowerCase().contains(query.toLowerCase()))
            .toList();
      });
    });
  }

  void _selectTrack(TrackInfo track) {
    setState(() {
      _selectedTrack = track;
      _searchCtrl.text = track.title;
      _showTrackList = false;
    });
    _updatePreview();
  }

  Future<void> _updatePreview() async {
    final track = _selectedTrack;
    if (track == null) return;
    setState(() => _previewLoading = true);
    try {
      final result = await BridgeService.instance.formatNamingPreview(
        pattern: _patternNotifier.value,
        title: track.title,
        artist: track.artistsString,
        albumArtist: track.artistsString,
        album: track.albumTitle,
        releaseDate: track.releaseDate,
        trackNumber: track.trackNum ?? 0,
        discNumber: track.discNum ?? 1,
        totalDiscs: track.discNum ?? 1,
      );
      if (!mounted) return;
      setState(() {
        _previewResult = result;
        _previewLoading = false;
      });
    } catch (_) {
      if (mounted) setState(() => _previewLoading = false);
    }
  }

  void _handleApply() {
    final pattern = _patternNotifier.value;
    SettingsManager.namingPattern.value = pattern;
    SettingsManager.syncNamingPatternToRust(pattern);
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colorScheme = Theme.of(context).colorScheme;

    return AlertDialog(
      titlePadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceSM,
        AppTheme.spaceSM,
        AppTheme.spaceSM,
      ),
      title: Row(
        children: [
          Expanded(
            child: Text('Naming Format Help', style: textTheme.titleMedium),
          ),
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      contentPadding: EdgeInsets.symmetric(horizontal: AppTheme.spaceLG),
      content: SizedBox(
        width: AppTheme.dialogWidthDesktop,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _buildPatternSelector(textTheme, colorScheme),
              SizedBox(height: AppTheme.spaceMD),
              _buildTrackSearch(textTheme, colorScheme),
              SizedBox(height: AppTheme.spaceSM),
              _buildPreview(textTheme, colorScheme),
              SizedBox(height: AppTheme.spaceMD),
              Divider(color: colorScheme.outlineVariant),
              SizedBox(height: AppTheme.spaceMD),
              _buildVariablesTable(textTheme, colorScheme),
              SizedBox(height: AppTheme.spaceMD),
              _buildConditionalsSection(textTheme, colorScheme),
              SizedBox(height: AppTheme.spaceLG),
            ],
          ),
        ),
      ),
      actionsPadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceSM,
        AppTheme.spaceLG,
        AppTheme.spaceSM,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
        ValueListenableBuilder<String>(
          valueListenable: _patternNotifier,
          builder: (context, localPattern, _) {
            final hasChanges =
                localPattern != SettingsManager.namingPattern.value;
            return FilledButton(
              onPressed: hasChanges ? _handleApply : null,
              child: const Text('Apply'),
            );
          },
        ),
      ],
    );
  }

  Widget _buildPatternSelector(TextTheme textTheme, ColorScheme colorScheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Naming Format', style: textTheme.titleSmall),
        SizedBox(height: AppTheme.spaceXS),
        TextField(
          controller: _patternCtrl,
          maxLines: null,
          style: textTheme.bodySmall?.copyWith(fontFamily: 'monospace'),
          decoration: InputDecoration(
            border: const OutlineInputBorder(),
            contentPadding: EdgeInsets.symmetric(
              horizontal: AppTheme.spaceSM,
              vertical: AppTheme.spaceSM,
            ),
            suffixIcon: PopupMenuButton<MapEntry<String, String>>(
              icon: const Icon(Icons.arrow_drop_down),
              tooltip: 'Presets',
              onSelected: (entry) {
                _patternCtrl.text = entry.value;
                _patternNotifier.value = entry.value;
                _updatePreview();
              },
              itemBuilder: (_) => [
                PopupMenuItem(
                  value: const MapEntry(
                    'Picard-style',
                    '{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}',
                  ),
                  child: const Text('Picard-style'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    'Album/DirPrefix_TrackNo_Title',
                    '{album}{album_disambiguation? (|)}/{album_artist?|_}{album?|_}{disc_prefix}{track_padded}_{title}',
                  ),
                  child: const Text('Album/DirPrefix_TrackNo_Title'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    'Artist/Album/## - Title',
                    '{artist}/{album}/{track_padded} - {title}',
                  ),
                  child: const Text('Artist/Album/## - Title'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    'Artist/Album/## Title',
                    '{artist}/{album}/{track_padded} {title}',
                  ),
                  child: const Text('Artist/Album/## Title'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    'Artist - ## - Title',
                    '{artist} - {track_padded} - {title}',
                  ),
                  child: const Text('Artist - ## - Title'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    '## - Title',
                    '{track_padded} - {title}',
                  ),
                  child: const Text('## - Title'),
                ),
                PopupMenuItem(
                  value: const MapEntry(
                    'Artist - Album/## - Title',
                    '{artist} - {album}/{track_padded} - {title}',
                  ),
                  child: const Text('Artist - Album/## - Title'),
                ),
              ],
            ),
          ),
          onChanged: (v) {
            _patternNotifier.value = v;
            _updatePreview();
          },
        ),
      ],
    );
  }

  Widget _buildTrackSearch(TextTheme textTheme, ColorScheme colorScheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Track Preview', style: textTheme.titleSmall),
        SizedBox(height: AppTheme.spaceXS),
        TextField(
          controller: _searchCtrl,
          style: textTheme.bodyMedium,
          decoration: InputDecoration(
            hintText: 'Search track by title...',
            prefixIcon: const Icon(Icons.search, size: 20),
            border: const OutlineInputBorder(),
            isDense: true,
            suffixIcon: _loading
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: Padding(
                      padding: EdgeInsets.all(12),
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  )
                : null,
          ),
          onChanged: (v) {
            _showTrackList = true;
            _onSearchChanged(v);
          },
          onTap: () => setState(() => _showTrackList = true),
        ),
        if (_showTrackList && _filteredTracks.isNotEmpty)
          Container(
            constraints: const BoxConstraints(maxHeight: 160),
            margin: const EdgeInsets.only(top: 4),
            decoration: BoxDecoration(
              border: Border.all(color: colorScheme.outlineVariant),
              borderRadius: BorderRadius.circular(4),
            ),
            child: ListView.builder(
              shrinkWrap: true,
              itemCount: _filteredTracks.length > 20
                  ? 20
                  : _filteredTracks.length,
              itemBuilder: (context, i) {
                final t = _filteredTracks[i];
                final isSelected = _selectedTrack?.id == t.id;
                return ListTile(
                  dense: true,
                  selected: isSelected,
                  selectedTileColor: colorScheme.primaryContainer.withValues(
                    alpha: 0.4,
                  ),
                  title: Text(
                    t.title,
                    style: textTheme.bodySmall,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  subtitle: Text(
                    '${t.artistsString}${t.albumTitle.isNotEmpty ? ' — ${t.albumTitle}' : ''}',
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  leading: Text(
                    t.trackNum != null ? '${t.trackNum}.' : '',
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  onTap: () => _selectTrack(t),
                );
              },
            ),
          ),
        if (_showTrackList &&
            _filteredTracks.isEmpty &&
            _searchCtrl.text.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              'No matching tracks found',
              style: textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ),
      ],
    );
  }

  Widget _buildPreview(TextTheme textTheme, ColorScheme colorScheme) {
    if (_selectedTrack == null) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(
          'Add tracks to your library to see a preview',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
      );
    }

    final track = _selectedTrack!;
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colorScheme.tertiaryContainer.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: colorScheme.tertiary.withValues(alpha: 0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              Icon(Icons.music_note, size: 14, color: colorScheme.tertiary),
              SizedBox(width: 6),
              Text('Title:', style: textTheme.labelSmall),
              SizedBox(width: 4),
              Expanded(
                child: Text(
                  track.title,
                  style: textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          SizedBox(height: 2),
          Row(
            children: [
              Icon(Icons.person, size: 14, color: colorScheme.tertiary),
              SizedBox(width: 6),
              Text('Artist:', style: textTheme.labelSmall),
              SizedBox(width: 4),
              Expanded(
                child: Text(
                  track.artistsString,
                  style: textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          SizedBox(height: 2),
          Row(
            children: [
              Icon(Icons.album, size: 14, color: colorScheme.tertiary),
              SizedBox(width: 6),
              Text('Album:', style: textTheme.labelSmall),
              SizedBox(width: 4),
              Expanded(
                child: Text(
                  track.albumTitle,
                  style: textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          if (track.trackNum != null) ...[
            SizedBox(height: 2),
            Row(
              children: [
                SizedBox(width: 20),
                Text(
                  'Track ${track.trackNum}',
                  style: textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                if (track.discNum != null) ...[
                  SizedBox(width: 12),
                  Text(
                    'Disc ${track.discNum}',
                    style: textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ],
          SizedBox(height: 8),
          Divider(
            height: 1,
            color: colorScheme.tertiary.withValues(alpha: 0.3),
          ),
          SizedBox(height: 8),
          Row(
            children: [
              if (_previewLoading)
                const SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              else
                Icon(Icons.preview, size: 14, color: colorScheme.tertiary),
              SizedBox(width: 6),
              Expanded(
                child: Text(
                  _previewLoading ? 'Loading...' : _previewResult,
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colorScheme.tertiary,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildVariablesTable(TextTheme textTheme, ColorScheme colorScheme) {
    final vars = [
      ('{title}', 'Track title', 'Song Name'),
      ('{artist}', 'Track artist', 'Artist Name'),
      ('{album_artist}', 'Album artist', 'Various Artists'),
      ('{album}', 'Album name', 'Album Title'),
      ('{track_number}', 'Track number (raw)', '1'),
      ('{track_padded}', 'Track number (padded)', '01'),
      ('{disc_number}', 'Disc number (raw)', '1'),
      ('{disc_padded}', 'Disc number (padded)', '01'),
      ('{disc_prefix}', '"02-" for disc > 1, else empty', '"02-" or ""'),
      ('{year}', 'Year from release date', '2024'),
      ('{release_date}', 'Full release date', '2024-03-15'),
      ('{album_disambiguation}', 'Album edition', 'Deluxe Edition'),
      ('{total_discs}', 'Total discs in album', '2'),
      ('{multi_artist}', '"1" if multi-artist album', '"1" or ""'),
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Variables', style: textTheme.titleSmall),
        SizedBox(height: AppTheme.spaceXS),
        Container(
          decoration: BoxDecoration(
            border: Border.all(color: colorScheme.outlineVariant),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Column(
            children: vars.map((v) {
              return Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                decoration: BoxDecoration(
                  border: Border(
                    bottom: BorderSide(
                      color: colorScheme.outlineVariant,
                      width: 0.5,
                    ),
                  ),
                ),
                child: Row(
                  children: [
                    SizedBox(
                      width: 140,
                      child: Text(
                        v.$1,
                        style: textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    Expanded(child: Text(v.$2, style: textTheme.bodySmall)),
                    SizedBox(
                      width: 100,
                      child: Text(
                        v.$3,
                        style: textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                        textAlign: TextAlign.right,
                      ),
                    ),
                  ],
                ),
              );
            }).toList(),
          ),
        ),
      ],
    );
  }

  Widget _syntaxRow(TextTheme textTheme, String syntax, String desc) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 140,
          child: Text(
            syntax,
            style: textTheme.bodySmall?.copyWith(
              fontFamily: 'monospace',
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(child: Text(desc, style: textTheme.bodySmall)),
      ],
    );
  }

  Widget _buildConditionalsSection(
    TextTheme textTheme,
    ColorScheme colorScheme,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Syntax', style: textTheme.titleSmall),
        SizedBox(height: AppTheme.spaceXS),
        Container(
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _syntaxRow(
                textTheme,
                '{var?expr|suffix}',
                'If var truthy: evaluate expr, append suffix. '
                    'If expr empty, use value directly. '
                    'If expr has {...}, it is a sub-pattern.',
              ),
              SizedBox(height: 4),
              _syntaxRow(
                textTheme,
                '{var?prefix|suffix}',
                'Wrapping: prefix + value + suffix (no braces in expr).',
              ),
              SizedBox(height: 4),
              _syntaxRow(
                textTheme,
                '{var??fallback|suffix}',
                'Use var if non-empty (+ suffix), else evaluate fallback.',
              ),
              SizedBox(height: 4),
              _syntaxRow(
                textTheme,
                '{var>N?expr|suffix}',
                'Numeric comparison: truthy if value > N.',
              ),
            ],
          ),
        ),
        SizedBox(height: AppTheme.spaceSM),
        ValueListenableBuilder<String>(
          valueListenable: _patternNotifier,
          builder: (context, pattern, _) {
            return Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.5,
                ),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Current pattern',
                    style: textTheme.labelSmall?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  SizedBox(height: 4),
                  Text(
                    pattern,
                    style: textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            );
          },
        ),
        SizedBox(height: AppTheme.spaceSM),
        Text(
          'Escape: \\| for literal pipe. '
          'Use / in the pattern to create subdirectories.',
          style: textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}
