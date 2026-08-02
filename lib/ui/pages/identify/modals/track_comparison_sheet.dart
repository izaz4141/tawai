import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/models/identify_result.dart';
import 'package:tawai/ui/pages/identify/widgets/comparison_row.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/editable_cover.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/shimmer.dart';
import 'package:tawai/utils/settings.dart';

class ComparisonSheetResult {
  final String? title;
  final String? artist;
  final String? album;
  final int? discNum;
  final int? trackNum;
  final String? releaseDate;
  final String? mbidRecording;
  final String? mbidAlbum;
  final String? mbidArtist;
  final String? lyrics;
  final Uint8List? coverBytes;
  final bool applied;

  const ComparisonSheetResult({
    this.title,
    this.artist,
    this.album,
    this.discNum,
    this.trackNum,
    this.releaseDate,
    this.mbidRecording,
    this.mbidAlbum,
    this.mbidArtist,
    this.lyrics,
    this.coverBytes,
    this.applied = false,
  });
}

Future<ComparisonSheetResult?> showTrackComparisonSheet({
  required BuildContext context,
  required IdentifyController controller,
  required TrackInfo currentTrack,
  required ReleaseTrackInfo correctTrack,
  required IdentifyAlbumResult album,
  required String recordingArtist,
}) {
  return showModalBottomSheet<ComparisonSheetResult>(
    context: context,
    isScrollControlled: true,
    builder: (_) => _TrackComparisonSheet(
      controller: controller,
      currentTrack: currentTrack,
      correctTrack: correctTrack,
      album: album,
      recordingArtist: recordingArtist,
    ),
  );
}

class _TrackComparisonSheet extends StatefulWidget {
  final IdentifyController controller;
  final TrackInfo currentTrack;
  final ReleaseTrackInfo correctTrack;
  final IdentifyAlbumResult album;
  final String recordingArtist;
  const _TrackComparisonSheet({
    required this.controller,
    required this.currentTrack,
    required this.correctTrack,
    required this.album,
    required this.recordingArtist,
  });

  @override
  State<_TrackComparisonSheet> createState() => _TrackComparisonSheetState();
}

class _TrackComparisonSheetState extends State<_TrackComparisonSheet>
    with SingleTickerProviderStateMixin {
  late final TextEditingController _titleCtrl;
  late final TextEditingController _artistCtrl;
  late final TextEditingController _albumCtrl;
  late final TextEditingController _discNumCtrl;
  late final TextEditingController _trackNumCtrl;
  late final TextEditingController _lyricsCtrl;
  late final TextEditingController _yearCtrl;

  late final String _origTitle;
  late final String _origArtist;
  late final String _origAlbum;
  late final String _origDiscNum;
  late final String _origTrackNum;
  late final String _origLyrics;
  late final String _origDate;
  late final String? _origMbidAlbum;
  late final String? _origMbidArtist;

  String? _appliedRemoteRecId;
  String? _appliedRemoteAlbumId;
  String? _appliedRemoteArtistId;

  Uint8List? _coverBytes;
  Uint8List? _localCoverBytes;
  bool _downloadingCover = false;
  bool _applying = false;
  bool _lyricsLoading = false;
  late final AnimationController _shimmerController;

  String get _remoteTitle => widget.correctTrack.title.isNotEmpty
      ? widget.correctTrack.title
      : widget.currentTrack.title;
  String get _remoteArtist => widget.recordingArtist;
  String get _remoteAlbum => widget.album.albumTitle;
  String get _remoteDiscNum => widget.correctTrack.discNumber?.toString() ?? '';
  String get _remoteTrackNum => widget.correctTrack.position?.toString() ?? '';
  String get _remoteMbid => widget.correctTrack.id;
  String get _remoteLyrics => widget.correctTrack.lyrics ?? '';
  String _remoteLyricsResult = '';
  String get _remoteLyricsDisplay =>
      _remoteLyricsResult.isNotEmpty ? _remoteLyricsResult : _remoteLyrics;
  String get _remoteDate => widget.album.releaseDate ?? '';

  @override
  void initState() {
    super.initState();
    _titleCtrl = TextEditingController(text: widget.currentTrack.title);
    _artistCtrl = TextEditingController(
      text: widget.currentTrack.artistsString,
    );
    _albumCtrl = TextEditingController(text: widget.currentTrack.albumTitle);
    _discNumCtrl = TextEditingController(
      text: widget.currentTrack.discNum?.toString() ?? '',
    );
    _trackNumCtrl = TextEditingController(
      text: widget.currentTrack.trackNum?.toString() ?? '',
    );
    _lyricsCtrl = TextEditingController(text: widget.currentTrack.lyrics ?? '');
    _yearCtrl = TextEditingController(
      text: widget.currentTrack.releaseDate ?? '',
    );
    _origTitle = widget.currentTrack.title;
    _origArtist = widget.currentTrack.artistsString;
    _origAlbum = widget.currentTrack.albumTitle;
    _origDiscNum = widget.currentTrack.discNum?.toString() ?? '';
    _origTrackNum = widget.currentTrack.trackNum?.toString() ?? '';
    _origLyrics = widget.currentTrack.lyrics ?? '';
    _origDate = widget.currentTrack.releaseDate ?? '';
    _origMbidAlbum = widget.currentTrack.albumMbid;
    _origMbidArtist =
        widget.currentTrack.artistMbid ??
        widget.currentTrack.artists.firstOrNull?.mbid;

    _shimmerController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1500),
    )..repeat();

    _loadLocalCover();

    if (_remoteLyrics.isEmpty) {
      _lyricsLoading = true;
      widget.controller
          .fetchRemoteLyrics(
            title: widget.correctTrack.title,
            artist: widget.recordingArtist,
            album: widget.album.albumTitle,
            duration: widget.correctTrack.durationSecs,
            preferSync: SettingsManager.lyricsPrefersync.value,
          )
          .then((lyrics) {
            if (!mounted) return;
            setState(() {
              _lyricsLoading = false;
              if (lyrics != null && lyrics.isNotEmpty) {
                _remoteLyricsResult = lyrics;
                final idx = widget.album.releaseTracks.indexWhere(
                  (t) => t.id == widget.correctTrack.id,
                );
                if (idx >= 0) {
                  widget.album.releaseTracks[idx] = widget
                      .album
                      .releaseTracks[idx]
                      .copyWith(lyrics: () => lyrics);
                }
              }
            });
          });
    }
  }

  @override
  void dispose() {
    _titleCtrl.dispose();
    _artistCtrl.dispose();
    _albumCtrl.dispose();
    _discNumCtrl.dispose();
    _trackNumCtrl.dispose();
    _lyricsCtrl.dispose();
    _yearCtrl.dispose();
    _shimmerController.dispose();
    super.dispose();
  }

  void _fillMissing() {
    setState(() {
      if (_titleCtrl.text.isEmpty) _titleCtrl.text = _remoteTitle;
      if (_artistCtrl.text.isEmpty) _artistCtrl.text = _remoteArtist;
      if (_albumCtrl.text.isEmpty) _albumCtrl.text = _remoteAlbum;
      if (_discNumCtrl.text.isEmpty) _discNumCtrl.text = _remoteDiscNum;
      if (_trackNumCtrl.text.isEmpty) _trackNumCtrl.text = _remoteTrackNum;
      if (_lyricsCtrl.text.isEmpty) _lyricsCtrl.text = _remoteLyricsDisplay;
      if (_yearCtrl.text.isEmpty) _yearCtrl.text = _remoteDate;
      _appliedRemoteRecId ??= widget.correctTrack.id;
      _appliedRemoteAlbumId ??= widget.album.albumMbid;
      _appliedRemoteArtistId ??= widget.album.releaseArtistMbid;
    });
    if (_coverBytes == null) _downloadCover();
  }

  void _replaceAll() {
    setState(() {
      _titleCtrl.text = _remoteTitle;
      _artistCtrl.text = _remoteArtist;
      _albumCtrl.text = _remoteAlbum;
      _discNumCtrl.text = _remoteDiscNum;
      _trackNumCtrl.text = _remoteTrackNum;
      _lyricsCtrl.text = _remoteLyricsDisplay;
      _yearCtrl.text = _remoteDate;
      _appliedRemoteRecId = widget.correctTrack.id;
      _appliedRemoteAlbumId = widget.album.albumMbid;
      _appliedRemoteArtistId = widget.album.releaseArtistMbid;
    });
    _downloadCover();
  }

  Future<void> _loadLocalCover() async {
    final albumId = widget.currentTrack.albumId;
    final bytes = await widget.controller.loadLocalCover(albumId);
    if (mounted) setState(() => _localCoverBytes = bytes);
  }

  Future<void> _downloadCover() async {
    final albumMbid = widget.album.albumMbid;
    if (albumMbid == null) return;
    setState(() => _downloadingCover = true);
    final bytes = await widget.controller.downloadCover(albumMbid);
    if (mounted) setState(() {
      _downloadingCover = false;
      _coverBytes = bytes;
    });
  }

  Future<void> _apply() async {
    setState(() => _applying = true);
    try {
      final st = widget.currentTrack;

      final result = await widget.controller.applyIdentification(
        trackId: st.id,
        title: _titleCtrl.text,
        artist: _artistCtrl.text,
        artistMbid: _appliedRemoteArtistId,
        album: _albumCtrl.text,
        albumMbid: _appliedRemoteAlbumId,
        albumDisambiguation: widget.album.albumDisambiguation,
        releaseDate: _yearCtrl.text.isNotEmpty ? _yearCtrl.text : null,
        trackNum: _trackNumCtrl.text.isNotEmpty
            ? int.tryParse(_trackNumCtrl.text)
            : null,
        discNum: _discNumCtrl.text.isNotEmpty
            ? int.tryParse(_discNumCtrl.text)
            : null,
        mbidRecording: _appliedRemoteRecId,
        lyrics: _lyricsCtrl.text.isNotEmpty ? _lyricsCtrl.text : null,
        coverBytes: _coverBytes,
      );
      if (!mounted) return;
      if (result.success) {
        Navigator.of(context).pop(ComparisonSheetResult(
          applied: true,
          title: _titleCtrl.text,
          artist: _artistCtrl.text,
          album: _albumCtrl.text,
          discNum: _discNumCtrl.text.isNotEmpty
              ? int.tryParse(_discNumCtrl.text)
              : null,
          trackNum: _trackNumCtrl.text.isNotEmpty
              ? int.tryParse(_trackNumCtrl.text)
              : null,
          releaseDate: _yearCtrl.text.isNotEmpty ? _yearCtrl.text : null,
          mbidRecording: _appliedRemoteRecId,
          mbidAlbum: _appliedRemoteAlbumId,
          mbidArtist: _appliedRemoteArtistId,
          lyrics: _lyricsCtrl.text.isNotEmpty ? _lyricsCtrl.text : null,
          coverBytes: _coverBytes,
        ));
      } else {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed: ${result.error ?? "Unknown error"}'),
            backgroundColor: Theme.of(context).colorScheme.error,
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _applying = false);
    }
  }

  String _fmtDuration(double? secs) {
    if (secs == null) return '';
    final total = secs.round();
    return '${total ~/ 60}:${(total % 60).toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Sheet(
      draggable: true,
      initialChildSize: 0.75,
      minChildSize: 0.4,
      maxChildSize: 0.95,
      header: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppTheme.spaceLG,
          vertical: AppTheme.spaceSM,
        ),
        child: Row(
          children: [
            Expanded(
              child: Text('Track Comparison', style: textTheme.titleMedium),
            ),
            TextButton(
              onPressed: _fillMissing,
              child: const Text('Fill Missing'),
            ),
            const SizedBox(width: 4),
            TextButton(
              onPressed: () {
                showDialog(
                  context: context,
                  builder: (ctx) => AlertDialog(
                    title: const Text('Replace All'),
                    content: const Text(
                      'Replace all local values with MusicBrainz data?',
                    ),
                    actions: [
                      TextButton(
                        onPressed: () => Navigator.of(ctx).pop(),
                        child: const Text('Cancel'),
                      ),
                      FilledButton(
                        onPressed: () {
                          Navigator.of(ctx).pop();
                          _replaceAll();
                        },
                        child: const Text('Replace'),
                      ),
                    ],
                  ),
                );
              },
              child: const Text('Replace All'),
            ),
          ],
        ),
      ),
      bodyBuilder: (_) => Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceLG,
          AppTheme.spaceSM,
          AppTheme.spaceLG,
          AppTheme.spaceLG,
        ),
        child: Column(
          children: [
            _buildCoverRow(colors, textTheme),
            const SizedBox(height: AppTheme.spaceMD),
            _buildColumnHeaders(textTheme, colors),
            const Divider(height: AppTheme.spaceLG),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.only(right: AppTheme.spaceSM),
                child: Column(
                  children: [
                    ComparisonEditableRow(
                      label: 'Title',
                      controller: _titleCtrl,
                      remoteValue: _remoteTitle,
                      originalValue: _origTitle,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonEditableRow(
                      label: 'Artist',
                      controller: _artistCtrl,
                      remoteValue: _remoteArtist,
                      originalValue: _origArtist,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonEditableRow(
                      label: 'Album',
                      controller: _albumCtrl,
                      remoteValue: _remoteAlbum,
                      originalValue: _origAlbum,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonEditableRow(
                      label: 'Disc #',
                      controller: _discNumCtrl,
                      remoteValue: _remoteDiscNum,
                      originalValue: _origDiscNum,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonEditableRow(
                      label: 'Track #',
                      controller: _trackNumCtrl,
                      remoteValue: _remoteTrackNum,
                      originalValue: _origTrackNum,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonEditableRow(
                      label: 'Date',
                      controller: _yearCtrl,
                      remoteValue: _remoteDate,
                      originalValue: _origDate,
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    _readonlyRow(
                      textTheme,
                      colors,
                      'Duration',
                      _fmtDuration(widget.currentTrack.durationSecs),
                      _fmtDuration(widget.correctTrack.durationSecs),
                    ),
                    ComparisonReadonlyRow(
                      label: 'MBID Rec',
                      current:
                          _appliedRemoteRecId ??
                          widget.currentTrack.mbidRecording ??
                          '',
                      remote: _remoteMbid,
                      applied: _appliedRemoteRecId != null,
                      onApply: () => setState(
                        () => _appliedRemoteRecId = widget.correctTrack.id,
                      ),
                      onRevert: () => setState(() => _appliedRemoteRecId = null),
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonReadonlyRow(
                      label: 'MBID Album',
                      current: _appliedRemoteAlbumId ?? _origMbidAlbum ?? '',
                      remote: widget.album.albumMbid ?? '',
                      applied: _appliedRemoteAlbumId != null,
                      onApply: () => setState(
                        () => _appliedRemoteAlbumId = widget.album.albumMbid,
                      ),
                      onRevert: () => setState(() => _appliedRemoteAlbumId = null),
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    ComparisonReadonlyRow(
                      label: 'MBID Artist',
                      current: _appliedRemoteArtistId ?? _origMbidArtist ?? '',
                      remote: widget.album.releaseArtistMbid ?? '',
                      applied: _appliedRemoteArtistId != null,
                      onApply: () => setState(
                        () =>
                            _appliedRemoteArtistId = widget.album.releaseArtistMbid,
                      ),
                      onRevert: () => setState(() => _appliedRemoteArtistId = null),
                      textTheme: textTheme,
                      colors: colors,
                    ),
                    _lyricsRow(textTheme, colors, _lyricsCtrl),
                  ],
                ),
              ),
            ),
            const Divider(height: AppTheme.spaceSM),
            SafeArea(
              child: SizedBox(
                width: double.infinity,
                child: FilledButton.icon(
                  onPressed: _applying ? null : _apply,
                  icon: _applying
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Icon(Icons.save),
                  label: Text(_applying ? 'Applying...' : 'Apply'),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildCoverRow(ColorScheme colors, TextTheme textTheme) {
    return Row(
      children: [
        Expanded(
          child: Column(
            children: [
              EditableCover(
                coverBytes: _coverBytes ?? _localCoverBytes,
                replaced: _coverBytes != null,
                onPickCover: _downloadCover,
                onRevertCover: _coverBytes != null
                    ? () => setState(() => _coverBytes = null)
                    : null,
                width: 120,
                height: 120,
              ),
              const SizedBox(height: 4),
              Text(
                'Local',
                style: textTheme.labelSmall?.copyWith(color: colors.primary),
              ),
            ],
          ),
        ),
        const SizedBox(width: 4),
        IconButton(
          onPressed: _downloadingCover
              ? null
              : (_coverBytes != null
                    ? () => setState(() => _coverBytes = null)
                    : _downloadCover),
          icon: _downloadingCover
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Icon(
                  _coverBytes != null ? Icons.undo : Icons.arrow_back,
                  color: _coverBytes != null ? colors.tertiary : colors.primary,
                ),
          tooltip: _coverBytes != null
              ? 'Revert cover'
              : 'Apply MusicBrainz cover',
        ),
        const SizedBox(width: 4),
        Expanded(
          child: Column(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: widget.album.albumMbid != null
                    ? Image.network(
                        'https://coverartarchive.org/release/${widget.album.albumMbid}/front-250.jpg',
                        width: 120,
                        height: 120,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) => Container(
                          width: 120,
                          height: 120,
                          color: colors.surfaceContainerHighest,
                          child: Icon(
                            Icons.album,
                            size: 48,
                            color: colors.onSurfaceVariant,
                          ),
                        ),
                      )
                    : Container(
                        width: 120,
                        height: 120,
                        color: colors.surfaceContainerHighest,
                        child: Icon(
                          Icons.album,
                          size: 48,
                          color: colors.onSurfaceVariant,
                        ),
                      ),
              ),
              const SizedBox(height: 4),
              Text(
                'MusicBrainz',
                style: textTheme.labelSmall?.copyWith(color: colors.tertiary),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildColumnHeaders(TextTheme textTheme, ColorScheme colors) {
    return Row(
      children: [
        const SizedBox(width: 64),
        Expanded(
          child: Text(
            'Local File',
            style: textTheme.labelSmall?.copyWith(
              color: colors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        const SizedBox(width: 36),
        Expanded(
          child: Text(
            'MusicBrainz',
            style: textTheme.labelSmall?.copyWith(
              color: colors.tertiary,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ],
    );
  }

  Widget _readonlyRow(
    TextTheme textTheme,
    ColorScheme colors,
    String label,
    String current,
    String remote,
  ) {
    return ComparisonReadonlyRow(
      label: label,
      current: current,
      remote: remote,
      textTheme: textTheme,
      colors: colors,
    );
  }

  Widget _lyricsRow(
    TextTheme textTheme,
    ColorScheme colors,
    TextEditingController ctrl,
  ) {
    final localValue = ctrl.text;
    final diff = localValue != _remoteLyricsDisplay;
    final applied = !diff && localValue != _origLyrics;

    IconData icon;
    Color iconColor;
    VoidCallback? onPress;
    String tooltip;

    if (diff) {
      icon = Icons.arrow_back;
      iconColor = colors.primary;
      onPress = () => setState(() => ctrl.text = _remoteLyricsDisplay);
      tooltip = 'Use LRCLIB lyrics';
    } else if (applied) {
      icon = Icons.undo;
      iconColor = colors.tertiary;
      onPress = () => setState(() => ctrl.text = _origLyrics);
      tooltip = 'Revert to original';
    } else {
      icon = Icons.arrow_back;
      iconColor = colors.outline.withAlpha(80);
      tooltip = '';
    }

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const SizedBox(
            width: 64,
            child: Text(
              'Lyrics',
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
            ),
          ),
          Expanded(
            child: TextField(
              controller: ctrl,
              style: textTheme.bodySmall,
              maxLines: null,
              minLines: 1,
              scrollPadding: EdgeInsets.zero,
              decoration: const InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(
                  horizontal: 8,
                  vertical: 6,
                ),
                border: OutlineInputBorder(),
              ),
            ),
          ),
          SizedBox(
            width: 36,
            child: IconButton(
              onPressed: onPress,
              icon: Icon(icon, size: 18, color: iconColor),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(),
              tooltip: tooltip,
            ),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 6),
              child: _lyricsLoading
                  ? ShimmerWidget(height: 68, controller: _shimmerController)
                  : SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: Text(_remoteLyricsDisplay, style: textTheme.bodySmall),
                    ),
            ),
          ),
        ],
      ),
    );
  }
}
