import 'dart:async';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/models/identify_result.dart';
import 'package:tawai/ui/pages/identify/widgets/comparison_row.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/cached_network_image.dart';
import 'package:tawai/ui/widgets/components/editable_cover.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/shimmer.dart';
import 'package:tawai/utils/bridge_service.dart';
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

/// One row of the comparison sheet. Concrete variants are rendered by the
/// sheet and, for [_EditableField], driven by Fill Missing / Replace All.
sealed class _ComparisonRow {}

class _SectionHeader extends _ComparisonRow {
  final String title;
  _SectionHeader(this.title);
}

class _EditableField extends _ComparisonRow {
  final String label;
  final TextEditingController controller;
  final String Function() remote;
  final String original;

  _EditableField({
    required this.label,
    required this.controller,
    required this.remote,
    required this.original,
  });
}

class _ReadonlyField extends _ComparisonRow {
  final String label;
  final String Function() current;
  final String Function() remote;

  _ReadonlyField({
    required this.label,
    required this.current,
    required this.remote,
  });
}

class _LyricsField extends _ComparisonRow {}

Future<ComparisonSheetResult?> showTrackComparisonSheet({
  required BuildContext context,
  required IdentifyController controller,
  required TrackInfo currentTrack,
  required ReleaseTrackInfo correctTrack,
  required IdentifyAlbumResult album,
  required String recordingArtist,
  AlbumInfo? albumInfo,
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
      albumInfo: albumInfo,
    ),
  );
}

class _TrackComparisonSheet extends StatefulWidget {
  final IdentifyController controller;
  final TrackInfo currentTrack;
  final ReleaseTrackInfo correctTrack;
  final IdentifyAlbumResult album;
  final String recordingArtist;
  final AlbumInfo? albumInfo;
  const _TrackComparisonSheet({
    required this.controller,
    required this.currentTrack,
    required this.correctTrack,
    required this.album,
    required this.recordingArtist,
    this.albumInfo,
  });

  @override
  State<_TrackComparisonSheet> createState() => _TrackComparisonSheetState();
}

class _TrackComparisonSheetState extends State<_TrackComparisonSheet>
    with SingleTickerProviderStateMixin {
  late final TextEditingController _titleCtrl;
  late final TextEditingController _artistCtrl;
  late final TextEditingController _artistsCtrl;
  late final TextEditingController _albumCtrl;
  late final TextEditingController _albumArtistCtrl;
  late final TextEditingController _albumArtistsCtrl;
  late final TextEditingController _genreCtrl;
  late final TextEditingController _trackNumCtrl;
  late final TextEditingController _totalTracksCtrl;
  late final TextEditingController _discNumCtrl;
  late final TextEditingController _totalDiscsCtrl;
  late final TextEditingController _dateCtrl;
  late final TextEditingController _originalDateCtrl;
  late final TextEditingController _mediaCtrl;
  late final TextEditingController _scriptCtrl;
  late final TextEditingController _isrcCtrl;
  late final TextEditingController _recordLabelCtrl;
  late final TextEditingController _releaseCountryCtrl;
  late final TextEditingController _releaseTypeCtrl;
  late final TextEditingController _mbidRecordingCtrl;
  late final TextEditingController _mbidTrackCtrl;
  late final TextEditingController _mbidArtistCtrl;
  late final TextEditingController _mbidReleaseCtrl;
  late final TextEditingController _mbidAlbumArtistCtrl;
  late final TextEditingController _mbidReleaseGroupCtrl;
  late final TextEditingController _artistSortCtrl;
  late final TextEditingController _albumArtistSortCtrl;
  late final TextEditingController _commentCtrl;
  late final TextEditingController _copyrightCtrl;
  late final TextEditingController _lyricsCtrl;

  late final List<TextEditingController> _controllers;

  /// Controllers that feed the naming-pattern path preview.
  late final List<TextEditingController> _previewControllers;
  late final List<_ComparisonRow> _rows;

  late final String _origLyrics;

  String? _targetSourceId;

  Timer? _pathPreviewDebounce;
  String _pathPreview = '';
  int _previewSeq = 0;

  bool get _isDownloadFolder =>
      widget.currentTrack.sourceType == 'download_folder';

  Uint8List? _coverBytes;
  Uint8List? _localCoverBytes;
  String? _coverUrl;
  bool _downloadingCover = false;
  bool _applying = false;
  bool _lyricsLoading = false;
  late final AnimationController _shimmerController;

  String get _remoteTitle => widget.correctTrack.title.isNotEmpty
      ? widget.correctTrack.title
      : widget.currentTrack.title;
  String get _remoteArtist => widget.recordingArtist;
  String get _remoteAlbum => widget.album.albumTitle;
  String get _remoteAlbumArtist => widget.album.releaseArtist ?? '';
  String get _remoteDiscNum => widget.correctTrack.discNumber?.toString() ?? '';
  String get _remoteTrackNum => widget.correctTrack.position?.toString() ?? '';
  String get _remoteMbid => widget.correctTrack.id;
  String get _remoteReleaseMbid => widget.album.albumMbid ?? '';
  String get _remoteAlbumArtistMbid => widget.album.releaseArtistMbid ?? '';
  String get _remoteLyrics => widget.correctTrack.lyrics ?? '';
  String _remoteLyricsResult = '';
  String get _remoteLyricsDisplay =>
      _remoteLyricsResult.isNotEmpty ? _remoteLyricsResult : _remoteLyrics;
  String get _remoteDate => widget.album.releaseDate ?? '';
  String get _remoteTotalTracks => widget.album.totalTracks?.toString() ?? '';
  String get _remoteTotalDiscs => widget.album.totalDiscs?.toString() ?? '';
  String get _remoteOriginalDate => widget.album.originalDate ?? '';
  String get _remoteMedia => widget.album.media ?? '';
  String get _remoteScript => widget.album.script ?? '';
  String get _remoteRecordLabel => widget.album.recordLabel ?? '';
  String get _remoteReleaseCountry => widget.album.releaseCountry ?? '';
  String get _remoteReleaseType => widget.album.releaseType ?? '';
  String get _remoteReleaseGroupMbid => widget.album.releaseGroupMbid ?? '';
  String get _remoteArtists {
    final joined = _joinNames(widget.correctTrack.artists);
    return joined.isNotEmpty ? joined : _remoteArtist;
  }

  String get _remoteAlbumArtists {
    final joined = _joinNames(widget.album.releaseArtists);
    return joined.isNotEmpty ? joined : _remoteAlbumArtist;
  }

  String get _remoteGenre => widget.correctTrack.genres.isNotEmpty
      ? widget.correctTrack.genres.join('; ')
      : widget.album.genres.join('; ');
  String get _remoteIsrc => widget.correctTrack.isrcs.join('; ');
  String get _remoteTrackMbid => widget.correctTrack.mbidTrack ?? '';
  String get _remoteArtistMbid =>
      widget.correctTrack.artists.firstOrNull?.mbid ??
      widget.correctTrack.artists.firstOrNull?.id ??
      '';
  String get _remoteArtistSort =>
      widget.correctTrack.artists.firstOrNull?.sortName ?? '';
  String get _remoteAlbumArtistSort =>
      widget.album.releaseArtists.firstOrNull?.sortName ?? '';
  String get _albumDisambiguation =>
      widget.album.albumDisambiguation ??
      widget.albumInfo?.disambiguation ??
      '';

  String _joinNames(List<ArtistInfo> artists) =>
      artists.map((a) => a.name).where((n) => n.isNotEmpty).join('; ');

  String _fmtGain(double? gain) =>
      gain == null ? '' : '${gain.toStringAsFixed(2)} dB';

  @override
  void initState() {
    super.initState();
    final t = widget.currentTrack;
    final ai = widget.albumInfo;

    _titleCtrl = TextEditingController(text: t.title);
    _artistCtrl = TextEditingController(text: t.artistsString);
    _artistsCtrl = TextEditingController(text: _joinNames(t.artists));
    _albumCtrl = TextEditingController(text: t.albumTitle);
    _albumArtistCtrl = TextEditingController(text: ai?.artistsString ?? '');
    _albumArtistsCtrl = TextEditingController(
      text: _joinNames(ai?.artists ?? const []),
    );
    _genreCtrl = TextEditingController(text: t.genres.join('; '));
    _trackNumCtrl = TextEditingController(text: t.trackNum?.toString() ?? '');
    _totalTracksCtrl = TextEditingController(
      text: (ai != null && ai.totalTracks > 0) ? ai.totalTracks.toString() : '',
    );
    _discNumCtrl = TextEditingController(text: t.discNum?.toString() ?? '');
    _totalDiscsCtrl = TextEditingController(
      text: (ai != null && ai.totalDiscs > 0) ? ai.totalDiscs.toString() : '',
    );
    _dateCtrl = TextEditingController(text: t.releaseDate ?? '');
    _originalDateCtrl = TextEditingController(text: ai?.originalDate ?? '');
    _mediaCtrl = TextEditingController(text: ai?.media ?? '');
    _scriptCtrl = TextEditingController(text: t.script ?? '');
    _isrcCtrl = TextEditingController(text: t.isrc ?? '');
    _recordLabelCtrl = TextEditingController(text: ai?.recordLabel ?? '');
    _releaseCountryCtrl = TextEditingController(text: ai?.releaseCountry ?? '');
    _releaseTypeCtrl = TextEditingController(text: ai?.releaseType ?? '');
    _mbidRecordingCtrl = TextEditingController(text: t.mbidRecording ?? '');
    _mbidTrackCtrl = TextEditingController(text: t.mbidTrack ?? '');
    _mbidArtistCtrl = TextEditingController(
      text: t.artistMbid ?? t.artists.firstOrNull?.mbid ?? '',
    );
    _mbidReleaseCtrl = TextEditingController(text: t.releaseMbid ?? '');
    _mbidAlbumArtistCtrl = TextEditingController(
      text: ai?.releaseArtistMbid ?? widget.album.releaseArtistMbid ?? '',
    );
    _mbidReleaseGroupCtrl = TextEditingController(
      text: ai?.releaseGroupMbid ?? '',
    );
    _artistSortCtrl = TextEditingController(
      text: t.artists.firstOrNull?.sortName ?? '',
    );
    _albumArtistSortCtrl = TextEditingController(
      text: ai?.artists.firstOrNull?.sortName ?? '',
    );
    _commentCtrl = TextEditingController(text: t.comment ?? '');
    _copyrightCtrl = TextEditingController(text: t.copyright ?? '');
    _lyricsCtrl = TextEditingController(text: t.lyrics ?? '');
    _origLyrics = t.lyrics ?? '';

    _controllers = [
      _titleCtrl,
      _artistCtrl,
      _artistsCtrl,
      _albumCtrl,
      _albumArtistCtrl,
      _albumArtistsCtrl,
      _genreCtrl,
      _trackNumCtrl,
      _totalTracksCtrl,
      _discNumCtrl,
      _totalDiscsCtrl,
      _dateCtrl,
      _originalDateCtrl,
      _mediaCtrl,
      _scriptCtrl,
      _isrcCtrl,
      _recordLabelCtrl,
      _releaseCountryCtrl,
      _releaseTypeCtrl,
      _mbidRecordingCtrl,
      _mbidTrackCtrl,
      _mbidArtistCtrl,
      _mbidReleaseCtrl,
      _mbidAlbumArtistCtrl,
      _mbidReleaseGroupCtrl,
      _artistSortCtrl,
      _albumArtistSortCtrl,
      _commentCtrl,
      _copyrightCtrl,
      _lyricsCtrl,
    ];

    _EditableField edit(
      String label,
      TextEditingController controller,
      String Function() remote,
    ) => _EditableField(
      label: label,
      controller: controller,
      remote: remote,
      original: controller.text,
    );

    _ReadonlyField ro(
      String label,
      String Function() current, [
      String Function()? remote,
    ]) => _ReadonlyField(
      label: label,
      current: current,
      remote: remote ?? () => '',
    );

    _rows = [
      _SectionHeader('Basic'),
      edit('Title', _titleCtrl, () => _remoteTitle),
      edit('Artist', _artistCtrl, () => _remoteArtist),
      edit('Artists', _artistsCtrl, () => _remoteArtists),
      edit('Album', _albumCtrl, () => _remoteAlbum),
      edit('Album Artist', _albumArtistCtrl, () => _remoteAlbumArtist),
      edit('Album Artists', _albumArtistsCtrl, () => _remoteAlbumArtists),
      edit('Genre(s)', _genreCtrl, () => _remoteGenre),

      _SectionHeader('Track / Release'),
      edit('Track #', _trackNumCtrl, () => _remoteTrackNum),
      edit('Total Tracks', _totalTracksCtrl, () => _remoteTotalTracks),
      edit('Disc #', _discNumCtrl, () => _remoteDiscNum),
      edit('Total Discs', _totalDiscsCtrl, () => _remoteTotalDiscs),
      edit('Date', _dateCtrl, () => _remoteDate),
      edit('Original Date', _originalDateCtrl, () => _remoteOriginalDate),
      edit('Media', _mediaCtrl, () => _remoteMedia),
      edit('Script', _scriptCtrl, () => _remoteScript),
      ro(
        'Length',
        () => _fmtDuration(widget.currentTrack.durationSecs),
        () => _fmtDuration(widget.correctTrack.durationSecs),
      ),

      _SectionHeader('Release / Identifiers'),
      edit('Record Label', _recordLabelCtrl, () => _remoteRecordLabel),
      edit('Release Country', _releaseCountryCtrl, () => _remoteReleaseCountry),
      edit('Release Type', _releaseTypeCtrl, () => _remoteReleaseType),
      edit('ISRC', _isrcCtrl, () => _remoteIsrc),
      edit('Recording MBID', _mbidRecordingCtrl, () => _remoteMbid),
      edit('Track MBID', _mbidTrackCtrl, () => _remoteTrackMbid),
      edit('Artist MBID', _mbidArtistCtrl, () => _remoteArtistMbid),
      edit('Release MBID', _mbidReleaseCtrl, () => _remoteReleaseMbid),
      edit(
        'Album Artist MBID',
        _mbidAlbumArtistCtrl,
        () => _remoteAlbumArtistMbid,
      ),
      edit(
        'Release Group MBID',
        _mbidReleaseGroupCtrl,
        () => _remoteReleaseGroupMbid,
      ),

      _SectionHeader('Sorting'),
      edit('Artist Sort', _artistSortCtrl, () => _remoteArtistSort),
      edit(
        'Album Artist Sort',
        _albumArtistSortCtrl,
        () => _remoteAlbumArtistSort,
      ),

      _SectionHeader('ReplayGain'),
      ro('Track Gain', () => _fmtGain(widget.currentTrack.trackGain)),
      ro('Track Peak', () => widget.currentTrack.trackPeak?.toString() ?? ''),

      _SectionHeader('Lyrics / Comment / Copyright'),
      _LyricsField(),
      edit('Comment', _commentCtrl, () => ''),
      edit('Copyright', _copyrightCtrl, () => ''),

      _SectionHeader('Technical'),
      ro('File Path', () => widget.currentTrack.filePath, () => _pathPreview),
      ro('Encoder', () => widget.currentTrack.encoder ?? ''),
    ];

    if (_isDownloadFolder && widget.controller.librarySources.isNotEmpty) {
      _targetSourceId = widget.controller.librarySources.first.id;
    }

    // Recompute the destination preview as the pattern-relevant fields change.
    _previewControllers = [
      _titleCtrl,
      _artistCtrl,
      _albumArtistCtrl,
      _albumCtrl,
      _dateCtrl,
      _trackNumCtrl,
      _discNumCtrl,
      _totalDiscsCtrl,
    ];
    for (final ctrl in _previewControllers) {
      ctrl.addListener(_schedulePathPreview);
    }
    _updatePathPreview();

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
    _pathPreviewDebounce?.cancel();
    for (final ctrl in _previewControllers) {
      ctrl.removeListener(_schedulePathPreview);
    }
    for (final ctrl in _controllers) {
      ctrl.dispose();
    }
    _shimmerController.dispose();
    super.dispose();
  }

  void _fillMissing() {
    setState(() {
      for (final row in _rows) {
        if (row is _EditableField &&
            row.controller.text.isEmpty &&
            row.remote().isNotEmpty) {
          row.controller.text = row.remote();
        }
      }
      if (_lyricsCtrl.text.isEmpty) {
        _lyricsCtrl.text = _remoteLyricsDisplay;
      }
    });
    if (_coverBytes == null) _downloadCover();
  }

  void _replaceAll() {
    setState(() {
      for (final row in _rows) {
        // Only overwrite where a MusicBrainz reference exists; an empty
        // reference must not wipe a local value. Use the row's arrow to clear
        // a field explicitly.
        if (row is _EditableField && row.remote().isNotEmpty) {
          row.controller.text = row.remote();
        }
      }
      if (_remoteLyricsDisplay.isNotEmpty) {
        _lyricsCtrl.text = _remoteLyricsDisplay;
      }
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
    if (mounted) {
      setState(() {
        _downloadingCover = false;
        _coverBytes = bytes;
        _coverUrl = null;
      });
    }
  }

  Future<void> _pickCoverFromUrl() async {
    final url = await showCoverUrlDialog(context);
    if (url == null || !mounted) return;
    setState(() {
      _coverUrl = url;
      _coverBytes = null;
    });
  }

  String? _textOrNull(String value) => value.isEmpty ? null : value;

  /// Debounce a preview recompute so rapid edits only trigger one preview.
  void _schedulePathPreview() {
    _pathPreviewDebounce?.cancel();
    _pathPreviewDebounce = Timer(
      const Duration(milliseconds: 200),
      _updatePathPreview,
    );
  }

  /// Recompute the destination preview from the currently edited values using
  /// the configured naming pattern. Mirrors the inputs `_apply` sends so the
  /// preview matches where `Apply` will place the file. The result is relative
  /// to the library source root (the root itself is resolved by the backend at
  /// apply time). No preview is shown when the track has no stored path.
  Future<void> _updatePathPreview() async {
    if (widget.currentTrack.filePath.isEmpty) {
      if (mounted && _pathPreview.isNotEmpty) {
        setState(() => _pathPreview = '');
      }
      return;
    }
    final seq = ++_previewSeq;
    try {
      final result = await BridgeService.instance.formatNamingPreview(
        pattern: SettingsManager.namingPattern.value,
        title: _titleCtrl.text,
        artist: _artistCtrl.text,
        albumArtist: _albumArtistCtrl.text.isNotEmpty
            ? _albumArtistCtrl.text
            : _artistCtrl.text,
        album: _albumCtrl.text,
        releaseDate: _textOrNull(_dateCtrl.text),
        trackNumber: int.tryParse(_trackNumCtrl.text) ?? 0,
        discNumber: int.tryParse(_discNumCtrl.text) ?? 1,
        albumDisambiguation: _albumDisambiguation,
        totalDiscs: int.tryParse(_totalDiscsCtrl.text) ?? 0,
      );
      if (!mounted || seq != _previewSeq) return;
      setState(() => _pathPreview = result);
    } catch (_) {
      if (!mounted || seq != _previewSeq) return;
      setState(() => _pathPreview = '');
    }
  }

  /// Splits a `;`-separated multi-value field into a list, or null when empty.
  List<String>? _splitList(String value) {
    final parts = value
        .split(';')
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    return parts.isEmpty ? null : parts;
  }

  Future<void> _apply() async {
    if (_isDownloadFolder && _targetSourceId == null) {
      if (!mounted) return;
      AppSnackBar.show(
        context,
        'Select a target library source to move the file into',
        type: SnackType.error,
      );
      return;
    }
    setState(() => _applying = true);
    try {
      final st = widget.currentTrack;

      // Track artist and album artist are distinct in ticket 07. Prefer an
      // edited track artist MBID, falling back to the album-artist MBID to
      // preserve the previous identification behaviour when only the release
      // artist MBID is known.
      final albumArtistMbid = _textOrNull(_mbidAlbumArtistCtrl.text);
      final artistMbid = _textOrNull(_mbidArtistCtrl.text) ?? albumArtistMbid;

      var coverBytes = _coverBytes;
      if (_coverUrl != null && coverBytes == null) {
        try {
          final resp = await http.get(Uri.parse(_coverUrl!));
          if (resp.statusCode == 200) {
            coverBytes = resp.bodyBytes;
          } else {
            throw 'Failed to download cover (HTTP ${resp.statusCode}).';
          }
        } catch (e) {
          if (!mounted) return;
          setState(() => _applying = false);
          AppSnackBar.show(
            context,
            'Could not download cover from URL: $e',
            type: SnackType.error,
          );
          return;
        }
      }

      final result = await widget.controller.applyIdentification(
        trackId: st.id,
        title: _titleCtrl.text,
        artist: _artistCtrl.text,
        artistMbid: artistMbid,
        albumArtist: _textOrNull(_albumArtistCtrl.text),
        albumArtistMbid: albumArtistMbid,
        artists: _splitList(_artistsCtrl.text),
        albumArtists: _splitList(_albumArtistsCtrl.text),
        artistSort: _textOrNull(_artistSortCtrl.text),
        albumArtistSort: _textOrNull(_albumArtistSortCtrl.text),
        album: _albumCtrl.text,
        albumMbid: _textOrNull(_mbidReleaseCtrl.text),
        albumDisambiguation: _albumDisambiguation,
        releaseDate: _textOrNull(_dateCtrl.text),
        originalDate: _textOrNull(_originalDateCtrl.text),
        releaseGroupMbid: _textOrNull(_mbidReleaseGroupCtrl.text),
        releaseCountry: _textOrNull(_releaseCountryCtrl.text),
        releaseType: _splitList(_releaseTypeCtrl.text),
        recordLabel: _textOrNull(_recordLabelCtrl.text),
        media: _textOrNull(_mediaCtrl.text),
        trackNum: int.tryParse(_trackNumCtrl.text),
        discNum: int.tryParse(_discNumCtrl.text),
        totalTracks: int.tryParse(_totalTracksCtrl.text),
        totalDiscs: int.tryParse(_totalDiscsCtrl.text),
        mbidRecording: _textOrNull(_mbidRecordingCtrl.text),
        mbidTrack: _textOrNull(_mbidTrackCtrl.text),
        isrc: _textOrNull(_isrcCtrl.text),
        comment: _textOrNull(_commentCtrl.text),
        copyright: _textOrNull(_copyrightCtrl.text),
        script: _textOrNull(_scriptCtrl.text),
        lyrics: _textOrNull(_lyricsCtrl.text),
        genres: _splitList(_genreCtrl.text),
        coverBytes: coverBytes,
        filePath: _isDownloadFolder ? st.filePath : null,
        targetSourceId: _isDownloadFolder ? _targetSourceId : null,
      );
      if (!mounted) return;
      if (result.success) {
        if (_isDownloadFolder && _targetSourceId != null) {
          widget.controller.applyDownloadScan(_targetSourceId!);
        }
        Navigator.of(context).pop(
          ComparisonSheetResult(
            applied: true,
            title: _titleCtrl.text,
            artist: _artistCtrl.text,
            album: _albumCtrl.text,
            discNum: int.tryParse(_discNumCtrl.text),
            trackNum: int.tryParse(_trackNumCtrl.text),
            releaseDate: _textOrNull(_dateCtrl.text),
            mbidRecording: _textOrNull(_mbidRecordingCtrl.text),
            mbidAlbum: _textOrNull(_mbidReleaseCtrl.text),
            mbidArtist: albumArtistMbid,
            lyrics: _textOrNull(_lyricsCtrl.text),
            coverBytes: coverBytes,
          ),
        );
      } else {
        AppSnackBar.show(
          context,
          'Failed: ${result.error ?? "Unknown error"}',
          type: SnackType.error,
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

  Widget _buildTargetSourcePicker(ColorScheme colors, TextTheme textTheme) {
    final sources = widget.controller.librarySources;
    if (sources.isEmpty) {
      return Padding(
        padding: EdgeInsets.symmetric(
          vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
        ),
        child: Text(
          'No library sources configured to move files into.',
          style: textTheme.bodySmall?.copyWith(color: colors.error),
        ),
      );
    }
    return Padding(
      padding: EdgeInsets.symmetric(
        vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
      ),
      child: DropdownButtonFormField<String>(
        value: _targetSourceId,
        decoration: InputDecoration(
          labelText: 'Move into library source',
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(AppTheme.radiusMD),
          ),
          isDense: true,
        ),
        items: sources
            .map((s) => DropdownMenuItem(value: s.id, child: Text(s.name)))
            .toList(),
        onChanged: (v) => setState(() => _targetSourceId = v),
      ),
    );
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
            SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
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
                    for (final row in _rows) _buildRow(row, textTheme, colors),
                    if (_isDownloadFolder)
                      _buildTargetSourcePicker(colors, textTheme),
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
                      ? SizedBox(
                          width:
                              AppTheme.spaceLG * AppTheme.spaceScale(context),
                          height:
                              AppTheme.spaceLG * AppTheme.spaceScale(context),
                          child: const CircularProgressIndicator(
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

  Widget _buildRow(
    _ComparisonRow row,
    TextTheme textTheme,
    ColorScheme colors,
  ) {
    switch (row) {
      case _SectionHeader():
        return _sectionHeader(row.title, textTheme, colors);
      case _EditableField():
        return ComparisonEditableRow(
          label: row.label,
          controller: row.controller,
          remoteValue: row.remote(),
          originalValue: row.original,
          textTheme: textTheme,
          colors: colors,
        );
      case _ReadonlyField():
        return ComparisonReadonlyRow(
          label: row.label,
          current: row.current(),
          remote: row.remote(),
          textTheme: textTheme,
          colors: colors,
        );
      case _LyricsField():
        return _lyricsRow(textTheme, colors);
    }
  }

  Widget _sectionHeader(String title, TextTheme textTheme, ColorScheme colors) {
    return Padding(
      padding: EdgeInsets.only(
        top: AppTheme.spaceMD * AppTheme.spaceScale(context),
        bottom: AppTheme.spaceXS * AppTheme.spaceScale(context),
      ),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          title,
          style: textTheme.labelMedium?.copyWith(
            color: colors.onSurfaceVariant,
            fontWeight: FontWeight.w700,
          ),
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
                coverUrl: _coverUrl,
                replaced: _coverBytes != null || _coverUrl != null,
                onPickCover: _downloadCover,
                onRevertCover: _coverBytes != null || _coverUrl != null
                    ? () => setState(() {
                        _coverBytes = null;
                        _coverUrl = null;
                      })
                    : null,
                onPickCoverFromUrl: _pickCoverFromUrl,
                width: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
                height: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
              ),
              SizedBox(height: AppTheme.spaceXS * AppTheme.spaceScale(context)),
              Text(
                'Local',
                style: textTheme.labelSmall?.copyWith(color: colors.primary),
              ),
            ],
          ),
        ),
        SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
        IconButton(
          onPressed: _downloadingCover
              ? null
              : (_coverBytes != null || _coverUrl != null
                    ? () => setState(() {
                        _coverBytes = null;
                        _coverUrl = null;
                      })
                    : _downloadCover),
          icon: _downloadingCover
              ? SizedBox(
                  width: AppTheme.spaceLG * AppTheme.spaceScale(context),
                  height: AppTheme.spaceLG * AppTheme.spaceScale(context),
                  child: const CircularProgressIndicator(strokeWidth: 2),
                )
              : Icon(
                  _coverBytes != null || _coverUrl != null
                      ? Icons.undo
                      : Icons.arrow_back,
                  color: _coverBytes != null || _coverUrl != null
                      ? colors.tertiary
                      : colors.primary,
                ),
          tooltip: _coverBytes != null || _coverUrl != null
              ? 'Revert cover'
              : 'Apply MusicBrainz cover',
        ),
        SizedBox(width: AppTheme.spaceXS * AppTheme.spaceScale(context)),
        Expanded(
          child: Column(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(
                  AppTheme.radiusSM * AppTheme.radiusScale(context),
                ),
                child: widget.album.albumMbid != null
                    ? CachedNetworkImage(
                        url:
                            'https://coverartarchive.org/release/${widget.album.albumMbid}/front-250.jpg',
                        width:
                            AppTheme.spaceMD *
                            10 *
                            AppTheme.spaceScale(context),
                        height:
                            AppTheme.spaceMD *
                            10 *
                            AppTheme.spaceScale(context),
                        fit: BoxFit.cover,
                        placeholder: Container(
                          width:
                              AppTheme.spaceMD *
                              10 *
                              AppTheme.spaceScale(context),
                          height:
                              AppTheme.spaceMD *
                              10 *
                              AppTheme.spaceScale(context),
                          color: colors.surfaceContainerHighest,
                          child: Icon(
                            Icons.album,
                            size: AppTheme.iconXL * AppTheme.iconScale(context),
                            color: colors.onSurfaceVariant,
                          ),
                        ),
                      )
                    : Container(
                        width:
                            AppTheme.spaceMD *
                            10 *
                            AppTheme.spaceScale(context),
                        height:
                            AppTheme.spaceMD *
                            10 *
                            AppTheme.spaceScale(context),
                        color: colors.surfaceContainerHighest,
                        child: Icon(
                          Icons.album,
                          size: AppTheme.iconXL * AppTheme.iconScale(context),
                          color: colors.onSurfaceVariant,
                        ),
                      ),
              ),
              SizedBox(height: AppTheme.spaceXS * AppTheme.spaceScale(context)),
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
        SizedBox(width: AppTheme.spaceXXL * 2 * AppTheme.spaceScale(context)),
        Expanded(
          child: Text(
            'Local File',
            style: textTheme.labelSmall?.copyWith(
              color: colors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        SizedBox(width: AppTheme.spaceLG * 2 * AppTheme.spaceScale(context)),
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

  Widget _lyricsRow(TextTheme textTheme, ColorScheme colors) {
    final localValue = _lyricsCtrl.text;
    final diff = localValue != _remoteLyricsDisplay;
    final applied = !diff && localValue != _origLyrics;

    IconData icon;
    Color iconColor;
    VoidCallback? onPress;
    String tooltip;

    if (diff) {
      icon = Icons.arrow_back;
      iconColor = colors.primary;
      onPress = () => setState(() => _lyricsCtrl.text = _remoteLyricsDisplay);
      tooltip = 'Use LRCLIB lyrics';
    } else if (applied) {
      icon = Icons.undo;
      iconColor = colors.tertiary;
      onPress = () => setState(() => _lyricsCtrl.text = _origLyrics);
      tooltip = 'Revert to original';
    } else {
      icon = Icons.arrow_back;
      iconColor = colors.outline.withAlpha(80);
      tooltip = '';
    }

    return Padding(
      padding: EdgeInsets.symmetric(
        vertical: AppTheme.spaceXS * AppTheme.spaceScale(context),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: AppTheme.spaceXXL * 2 * AppTheme.spaceScale(context),
            child: Text(
              'Lyrics',
              style: TextStyle(
                fontSize: AppTheme.textSM * AppTheme.textScale(context),
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          Expanded(
            child: TextField(
              controller: _lyricsCtrl,
              style: textTheme.bodySmall,
              maxLines: null,
              minLines: 1,
              scrollPadding: EdgeInsets.zero,
              decoration: InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(
                  horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
                  vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                border: const OutlineInputBorder(),
              ),
            ),
          ),
          SizedBox(
            width: AppTheme.spaceLG * 2 * AppTheme.spaceScale(context),
            child: IconButton(
              onPressed: onPress,
              icon: Icon(
                icon,
                size: AppTheme.iconSM * AppTheme.iconScale(context),
                color: iconColor,
              ),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(),
              tooltip: tooltip,
            ),
          ),
          Expanded(
            child: Padding(
              padding: EdgeInsets.symmetric(
                vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
              ),
              child: _lyricsLoading
                  ? ShimmerWidget(
                      height:
                          AppTheme.spaceMD * 6 * AppTheme.spaceScale(context),
                      controller: _shimmerController,
                    )
                  : SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: Text(
                        _remoteLyricsDisplay,
                        style: textTheme.bodySmall,
                      ),
                    ),
            ),
          ),
        ],
      ),
    );
  }
}
