import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/utils/identify_helpers.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';

class MusicBrainzFetchResult {
  final String title;
  final String artist;
  final String? artistId;
  final String album;
  final String? albumId;
  final String? releaseDate;
  final int? trackNumber;
  final int? discNumber;
  final String? mbidRecording;
  final Uint8List? coverBytes;

  const MusicBrainzFetchResult({
    required this.title,
    required this.artist,
    this.artistId,
    required this.album,
    this.albumId,
    this.releaseDate,
    this.trackNumber,
    this.discNumber,
    this.mbidRecording,
    this.coverBytes,
  });
}

Future<MusicBrainzFetchResult?> showIdentifyDialog(
  BuildContext context, {
  String? currentTitle,
  String? currentArtist,
  String? currentAlbum,
  void Function(RecordingInfo recording, ReleaseInfo release)? onReleaseSelected,
}) {
  return showDialog<MusicBrainzFetchResult>(
    context: context,
    builder: (_) => _IdentifyDialog(
      currentTitle: currentTitle,
      currentArtist: currentArtist,
      currentAlbum: currentAlbum,
      onReleaseSelected: onReleaseSelected,
    ),
  );
}

enum _Stage { recordings, releases, tracks }

class _IdentifyDialog extends StatefulWidget {
  final String? currentTitle;
  final String? currentArtist;
  final String? currentAlbum;
  final void Function(RecordingInfo recording, ReleaseInfo release)? onReleaseSelected;

  const _IdentifyDialog({
    this.currentTitle,
    this.currentArtist,
    this.currentAlbum,
    this.onReleaseSelected,
  });

  static String _buildQuery(String? title, String? artist, String? album) {
    final parts = <String>[];
    if (title != null && title.isNotEmpty) {
      parts.add('recording:"${title.replaceAll('"', '\\"')}"');
    }
    if (artist != null && artist.isNotEmpty) {
      parts.add('artist:"${artist.replaceAll('"', '\\"')}"');
    }
    if (album != null && album.isNotEmpty) {
      parts.add('release:"${album.replaceAll('"', '\\"')}"');
    }
    return parts.isEmpty ? '' : parts.join(' AND ');
  }

  @override
  State<_IdentifyDialog> createState() => _IdentifyDialogState();
}

class _IdentifyDialogState extends State<_IdentifyDialog> {
  _Stage _stage = _Stage.recordings;

  final _searchCtrl = TextEditingController();
  List<RecordingInfo> _recordings = [];
  bool _searching = false;
  bool _hasAutoSearched = false;

  RecordingInfo? _selectedRecording;

  ReleaseInfo? _selectedRelease;
  List<ReleaseTrackInfo> _tracks = [];
  bool _loadingTracks = false;
  ReleaseTrackInfo? _bestTrack;

  String _formatDuration(double? secs) {
    if (secs == null) return '';
    final totalSecs = secs.round();
    final minutes = totalSecs ~/ 60;
    final seconds = totalSecs % 60;
    return '${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
  }

  @override
  void initState() {
    super.initState();
    final query = _IdentifyDialog._buildQuery(
      widget.currentTitle,
      widget.currentArtist,
      widget.currentAlbum,
    );
    if (query.isNotEmpty) {
      _searchCtrl.text = query;
      _hasAutoSearched = true;
      WidgetsBinding.instance.addPostFrameCallback((_) => _search());
    }
  }

  @override
  void dispose() {
    _searchCtrl.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    final query = _searchCtrl.text.trim();
    if (query.isEmpty) return;
    setState(() {
      _searching = true;
      _recordings = [];
    });
    final results = await BridgeService.instance.searchMusicBrainz(query);
    if (!mounted) return;
    setState(() {
      _recordings = results;
      _searching = false;
    });
  }

  void _selectRecording(RecordingInfo rec) {
    if (rec.releases.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No releases found for this recording.')),
      );
      return;
    }
    setState(() {
      _selectedRecording = rec;
      _stage = _Stage.releases;
    });
  }

  void _selectRelease(ReleaseInfo release) {
    final callback = widget.onReleaseSelected;
    if (callback != null) {
      callback(_selectedRecording!, release);
      Navigator.of(context).pop();
      return;
    }
    setState(() {
      _selectedRelease = release;
      _stage = _Stage.tracks;
      _loadingTracks = true;
    });
    _fetchTracks(release.id);
  }

  Future<void> _fetchTracks(String releaseId) async {
    final data = await BridgeService.instance.getReleaseTracks(releaseId);
    if (!mounted) return;
    final rec = _selectedRecording;
    setState(() {
      _tracks = data.tracks;
      _loadingTracks = false;
      _bestTrack = rec != null
          ? autoAssignCorrectTrack(
              trackTitle: rec.title,
              trackDuration: rec.durationSecs,
              trackPosition: null,
              releaseTracks: data.tracks,
            )
          : null;
    });
  }

  Future<void> _selectTrack(ReleaseTrackInfo track) async {
    final rec = _selectedRecording!;
    final release = _selectedRelease!;
    Uint8List? coverBytes;
    try {
      final url = Uri.parse(
        'https://coverartarchive.org/release/${release.id}/front-250.jpg',
      );
      final resp = await http.get(url);
      if (resp.statusCode == 200) {
        coverBytes = resp.bodyBytes;
      }
    } catch (_) {}
    if (!mounted) return;
    Navigator.of(context).pop(MusicBrainzFetchResult(
      title: track.title,
      artist: rec.artist,
      artistId: rec.artistId,
      album: release.title,
      albumId: release.id,
      releaseDate: release.date,
      trackNumber: track.position,
      discNumber: track.discNumber,
      mbidRecording: rec.id,
      coverBytes: coverBytes,
    ));
  }

  void _goBack() {
    switch (_stage) {
      case _Stage.releases:
        setState(() {
          _selectedRecording = null;
          _stage = _Stage.recordings;
        });
      case _Stage.tracks:
        setState(() {
          _selectedRelease = null;
          _tracks = [];
          _stage = _Stage.releases;
        });
      case _Stage.recordings:
        break;
    }
  }

  String _titleForStage() {
    switch (_stage) {
      case _Stage.recordings:
        return 'Search MusicBrainz';
      case _Stage.releases:
        return 'Choose Release';
      case _Stage.tracks:
        return 'Choose Track';
    }
  }

  @override
  Widget build(BuildContext context) {
    final isDesktop = AppTheme.isDesktop(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;

    return AlertDialog(
      titlePadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceLG,
        AppTheme.spaceLG,
        0,
      ),
      title: Row(
        children: [
          if (_stage != _Stage.recordings)
            IconButton(
              icon: const Icon(Icons.arrow_back),
              tooltip: 'Back',
              onPressed: _goBack,
            ),
          Expanded(
            child: Text(_titleForStage(), style: textTheme.titleMedium),
          ),
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      contentPadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceSM,
        AppTheme.spaceLG,
        0,
      ),
      content: SizedBox(
        width: isDesktop
            ? AppTheme.dialogWidthDesktop
            : AppTheme.dialogWidthMobile,
        height: isDesktop ? 480 : 400,
        child: _buildBody(colors, textTheme),
      ),
    );
  }

  Widget _buildBody(ColorScheme colors, TextTheme textTheme) {
    switch (_stage) {
      case _Stage.recordings:
        return _buildRecordingsStage(colors, textTheme);
      case _Stage.releases:
        return _buildReleasesStage(colors, textTheme);
      case _Stage.tracks:
        return _buildTracksStage(colors, textTheme);
    }
  }

  Widget _buildRecordingsStage(ColorScheme colors, TextTheme textTheme) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: AppTheme.spaceMD),
          child: TextField(
            controller: _searchCtrl,
            decoration: InputDecoration(
              hintText: 'Search recordings...',
              suffixIcon: _searching
                  ? Padding(
                      padding: const EdgeInsets.all(AppTheme.spaceMD),
                      child: SizedBox(
                        width: AppTheme.iconSM,
                        height: AppTheme.iconSM,
                        child: const CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : IconButton(
                      icon: const Icon(Icons.search),
                      tooltip: 'Search',
                      onPressed: _search,
                    ),
            ),
            onSubmitted: (_) => _search(),
          ),
        ),
        Expanded(
          child: _searching
              ? const Center(child: CircularProgressIndicator())
              : _recordings.isEmpty
                  ? Center(
                      child: Text(
                        _hasAutoSearched
                            ? 'No results found. Try a different query.'
                            : 'Search for a recording to get started.',
                        style: textTheme.bodyMedium,
                      ),
                    )
                  : ListView.builder(
                      itemCount: _recordings.length,
                      itemBuilder: (_, i) => Padding(
                        padding: const EdgeInsets.only(bottom: AppTheme.spaceXS),
                        child: _buildRecordingCard(_recordings[i], colors, textTheme),
                      ),
                    ),
        ),
      ],
    );
  }

  Widget _buildRecordingCard(
      RecordingInfo rec, ColorScheme colors, TextTheme textTheme) {
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(AppTheme.radiusMD),
        onTap: () => _selectRecording(rec),
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.spaceSM),
          child: Row(
            children: [
              rec.cover != null
                  ? ClipRRect(
                      borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                      child: Image.network(
                        rec.cover!,
                        width: AppTheme.iconXL,
                        height: AppTheme.iconXL,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) => Container(
                          width: AppTheme.iconXL,
                          height: AppTheme.iconXL,
                          color: colors.surfaceContainerHighest,
                          child: Icon(Icons.music_note,
                              size: AppTheme.iconMD,
                              color: colors.onSurfaceVariant),
                        ),
                      ),
                    )
                  : Container(
                      width: AppTheme.iconXL,
                      height: AppTheme.iconXL,
                      decoration: BoxDecoration(
                        color: colors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                      ),
                      child: Icon(
                        Icons.music_note,
                        size: AppTheme.iconMD,
                        color: colors.onSurfaceVariant,
                      ),
                    ),
              const SizedBox(width: AppTheme.spaceMD),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      rec.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      rec.artist,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodySmall,
                    ),
                    Row(
                      children: [
                        if (rec.releases.isNotEmpty)
                          Text(
                            '${rec.releases.length} release${rec.releases.length > 1 ? 's' : ''}',
                            style: textTheme.bodySmall?.copyWith(
                              color: colors.onSurfaceVariant,
                            ),
                          ),
                        if (rec.releases.isNotEmpty && rec.durationSecs != null)
                          const SizedBox(width: AppTheme.spaceSM),
                        if (rec.durationSecs != null)
                          Text(
                            _formatDuration(rec.durationSecs),
                            style: textTheme.bodySmall
                                ?.copyWith(color: colors.outline),
                          ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(width: AppTheme.spaceSM),
              Icon(Icons.arrow_forward_ios,
                  size: AppTheme.iconSM, color: colors.outline),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildReleasesStage(ColorScheme colors, TextTheme textTheme) {
    final rec = _selectedRecording!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.all(AppTheme.spaceSM),
          margin: const EdgeInsets.only(bottom: AppTheme.spaceMD),
          decoration: BoxDecoration(
            color: colors.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(AppTheme.radiusSM),
          ),
          child: Row(
            children: [
              Icon(Icons.music_note,
                  size: AppTheme.iconSM, color: colors.onSurfaceVariant),
              const SizedBox(width: AppTheme.spaceXS),
              Expanded(
                child: Text(
                  '${rec.title} — ${rec.artist}',
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView.builder(
            itemCount: rec.releases.length,
            itemBuilder: (_, i) => Padding(
              padding: const EdgeInsets.only(bottom: AppTheme.spaceXS),
              child: _buildReleaseCard(rec.releases[i], colors, textTheme),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildReleaseCard(
      ReleaseInfo release, ColorScheme colors, TextTheme textTheme) {
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(AppTheme.radiusMD),
        onTap: () => _selectRelease(release),
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.spaceSM),
          child: Row(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                child: Image.network(
                  'https://coverartarchive.org/release/${release.id}/front-250.jpg',
                  width: AppTheme.iconXL,
                  height: AppTheme.iconXL,
                  fit: BoxFit.cover,
                  errorBuilder: (_, _, _) => Container(
                    width: AppTheme.iconXL,
                    height: AppTheme.iconXL,
                    color: colors.surfaceContainerHighest,
                    child: Icon(
                      Icons.album,
                      size: AppTheme.iconMD,
                      color: colors.onSurfaceVariant,
                    ),
                  ),
                ),
              ),
              const SizedBox(width: AppTheme.spaceMD),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      release.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodyLarge
                          ?.copyWith(fontWeight: FontWeight.w600),
                    ),
                    if (release.date != null || release.country != null)
                      Text(
                        [
                          if (release.country != null) release.country!,
                          if (release.date != null) release.date!,
                        ].join(' · '),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.bodySmall
                            ?.copyWith(color: colors.onSurfaceVariant),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: AppTheme.spaceSM),
              Icon(Icons.arrow_forward_ios,
                  size: AppTheme.iconSM, color: colors.outline),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildTracksStage(ColorScheme colors, TextTheme textTheme) {
    final release = _selectedRelease!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.all(AppTheme.spaceSM),
          margin: const EdgeInsets.only(bottom: AppTheme.spaceMD),
          decoration: BoxDecoration(
            color: colors.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(AppTheme.radiusSM),
          ),
          child: Row(
            children: [
              Icon(Icons.album,
                  size: AppTheme.iconSM, color: colors.onSurfaceVariant),
              const SizedBox(width: AppTheme.spaceXS),
              Expanded(
                child: Text(
                  release.title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: _loadingTracks
              ? const Center(child: CircularProgressIndicator())
              : _tracks.isEmpty
                  ? Center(
                      child: Text(
                        'No tracks found.',
                        style: textTheme.bodyMedium,
                      ),
                    )
                  : ListView.builder(
                      itemCount: _tracks.length,
                      itemBuilder: (_, i) => Padding(
                        padding: const EdgeInsets.only(bottom: AppTheme.spaceXS),
                        child: _buildTrackRow(
                          _tracks[i], _tracks[i] == _bestTrack, colors, textTheme),
                      ),
                    ),
        ),
      ],
    );
  }

  Widget _buildTrackRow(
      ReleaseTrackInfo track, bool isBestMatch, ColorScheme colors, TextTheme textTheme) {
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(AppTheme.radiusMD),
        onTap: () => _selectTrack(track),
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: AppTheme.spaceSM,
            vertical: AppTheme.spaceSM + 2,
          ),
          child: Row(
            children: [
              if (track.position != null)
                SizedBox(
                  width: 32,
                  child: Text(
                    '${track.position}',
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.outline,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              const SizedBox(width: AppTheme.spaceSM),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      track.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    if (isBestMatch)
                      Text(
                        'Best match',
                        style: textTheme.labelSmall?.copyWith(
                          color: Colors.green,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                  ],
                ),
              ),
              if (track.durationSecs != null)
                Padding(
                  padding: const EdgeInsets.only(right: AppTheme.spaceSM),
                  child: Text(
                    _formatDuration(track.durationSecs),
                    style: textTheme.bodySmall?.copyWith(color: colors.outline),
                  ),
                ),
              Icon(
                isBestMatch ? Icons.check_circle : Icons.check_circle_outline,
                size: AppTheme.iconSM,
                color: isBestMatch ? Colors.green : colors.primary,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
