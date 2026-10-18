import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:tawai/models/audio_format.dart';
import 'package:tawai/models/editable_tag_data.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/editable_cover.dart';
import 'package:tawai/ui/widgets/components/media_uploader.dart';
import 'package:tawai/ui/widgets/dialog/identify_dialog.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/helper.dart';
import 'package:tawai/utils/io_service.dart';

Future<void> showTagEditorDialog(BuildContext context, {String? initialPath}) {
  return showDialog(
    context: context,
    builder: (_) => _TagEditorDialog(initialPath: initialPath),
  );
}

class _TagEditorDialog extends StatefulWidget {
  final String? initialPath;
  const _TagEditorDialog({this.initialPath});

  @override
  State<_TagEditorDialog> createState() => _TagEditorDialogState();
}

class _TagEditorDialogState extends State<_TagEditorDialog> {
  // Width reserved for field labels so all rows line up.
  static const double _labelWidth = 130;

  String? _path;
  String? _filename;
  Uint8List? _fileBytes;
  bool _loading = false;
  bool _applying = false;
  String? _error;
  ReadFileTagsResponse? _tags;
  Uint8List? _selectedCover;
  String? _coverUrl;
  bool _coverRemoved = false;

  /// One controller per editable field, keyed by the field's stable id.
  final Map<String, TextEditingController> _controllers = {};

  TextEditingController _ctrl(String key) =>
      _controllers.putIfAbsent(key, TextEditingController.new);

  @override
  void initState() {
    super.initState();
    if (widget.initialPath != null) {
      _path = widget.initialPath;
      _loadTags();
    }
  }

  @override
  void dispose() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  Future<UploadMediaResult> _handleUpload(UploadedMediaFile file) async {
    final path = file.path;
    if (path != null) {
      final error = await _readTagsForPath(path);
      if (!mounted) {
        return const UploadMediaResult(success: false, error: 'Dialog closed.');
      }
      if (error != null) {
        return UploadMediaResult(success: false, error: error);
      }
      setState(() => _path = path);
      return const UploadMediaResult(success: true);
    }
    final bytes = file.bytes;
    final name = file.name;
    if (bytes == null) {
      return const UploadMediaResult(
        success: false,
        error: 'File path is not available on this platform.',
      );
    }
    final error = await _readTagsForBytes(name, bytes);
    if (!mounted) {
      return const UploadMediaResult(success: false, error: 'Dialog closed.');
    }
    if (error != null) {
      return UploadMediaResult(success: false, error: error);
    }
    setState(() {
      _filename = name;
      _fileBytes = bytes;
    });
    return const UploadMediaResult(success: true);
  }

  Future<void> _pickCover() async {
    final result = await IOServiceFactory.create().pickFile(
      allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'],
    );
    final bytes = result?.bytes;
    if (bytes == null) return;
    setState(() {
      _selectedCover = bytes;
      _coverRemoved = false;
      _coverUrl = null;
    });
  }

  Future<void> _pickCoverFromUrl() async {
    final url = await showCoverUrlDialog(context);
    if (url == null || !mounted) return;
    setState(() {
      _coverUrl = url;
      _coverRemoved = false;
      _selectedCover = null;
    });
  }

  Future<void> _fetchFromMusicBrainz() async {
    final result = await showIdentifyDialog(
      context,
      currentTitle: _ctrl('title').text,
      currentArtist: _ctrl('artist').text,
      currentAlbum: _ctrl('album').text,
    );
    if (result == null || !mounted) return;
    setState(() {
      _ctrl('title').text = result.title;
      _ctrl('artist').text = result.artist;
      _ctrl('album').text = result.album;
      _ctrl('album_artist').text = result.artist;
      _ctrl('track').text = result.trackNumber?.toString() ?? '';
      _ctrl('disc').text = result.discNumber?.toString() ?? '';
      _ctrl('date').text = result.releaseDate ?? '';
      if (result.coverBytes != null) {
        _selectedCover = result.coverBytes;
        _coverRemoved = false;
        _coverUrl = null;
      }
    });
    AppSnackBar.show(
      context,
      'Metadata fetched from MusicBrainz.',
      type: SnackType.success,
    );
  }

  Future<void> _loadTags() async {
    if (_path == null) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    final error = await _readTagsForPath(_path!);
    if (!mounted) return;
    setState(() {
      _loading = false;
      _error = error;
    });
  }

  Future<String?> _readTagsForPath(String path) async {
    try {
      final response = await BridgeService.instance.readFileTags(path);
      if (!mounted) return 'Tag editor is not available on this platform.';
      if (response == null) {
        return 'Tag editor is not available on this platform.';
      }
      if (response.error != null) {
        return response.error;
      }
      _applyTagResponse(response);
      return null;
    } catch (e) {
      if (!mounted) return null;
      return e.toString();
    }
  }

  Future<String?> _readTagsForBytes(String filename, Uint8List bytes) async {
    try {
      final response = await BridgeService.instance.readFileTagsBytes(
        filename,
        bytes,
      );
      if (!mounted) return 'Tag editor is not available on this platform.';
      if (response == null) {
        return 'Tag editor is not available on this platform.';
      }
      if (response.error != null) {
        return response.error;
      }
      _applyTagResponse(response);
      return null;
    } catch (e) {
      if (!mounted) return null;
      return e.toString();
    }
  }

  void _applyTagResponse(ReadFileTagsResponse response) {
    setState(() {
      _tags = response;
      _selectedCover = null;
      _coverUrl = null;
      _coverRemoved = false;
    });
    _ctrl('title').text = response.title;
    _ctrl('artist').text = response.artist;
    _ctrl('artists').text = response.artists.join('; ');
    _ctrl('album').text = response.album;
    _ctrl('album_artist').text = response.albumArtist;
    _ctrl('album_artists').text = response.albumArtists.join('; ');
    _ctrl('genres').text = response.genres.join(', ');
    _ctrl('track').text = response.trackNumber?.toString() ?? '';
    _ctrl('total_tracks').text = response.totalTracks?.toString() ?? '';
    _ctrl('disc').text = response.discNumber?.toString() ?? '';
    _ctrl('total_discs').text = response.totalDiscs?.toString() ?? '';
    _ctrl('date').text = response.releaseDate ?? '';
    _ctrl('original_date').text = response.originalDate ?? '';
    _ctrl('media').text = response.media ?? '';
    _ctrl('script').text = response.script ?? '';
    _ctrl('record_label').text = response.recordLabel ?? '';
    _ctrl('release_country').text = response.releaseCountry ?? '';
    _ctrl('release_type').text = response.releaseType.join('; ');
    _ctrl('isrc').text = response.isrc ?? '';
    _ctrl('mbid_recording').text = response.mbidRecording ?? '';
    _ctrl('mbid_track').text = response.mbidTrack ?? '';
    _ctrl('mbid_artist').text = response.mbidArtist ?? '';
    _ctrl('mbid_release_artist').text = response.mbidReleaseArtist ?? '';
    _ctrl('mbid_release').text = response.mbidRelease ?? '';
    _ctrl('mbid_release_group').text = response.mbidReleaseGroup ?? '';
    _ctrl('artist_sort').text = response.artistSort;
    _ctrl('album_artist_sort').text = response.albumArtistSort;
    _ctrl('comment').text = response.comment ?? '';
    _ctrl('copyright').text = response.copyright ?? '';
    _ctrl('lyrics').text = response.lyrics ?? '';
  }

  Future<void> _apply() async {
    if (_tags == null) return;
    if (_path == null && (_filename == null || _fileBytes == null)) return;
    setState(() => _applying = true);
    try {
      var cover = _selectedCover;
      if (_coverUrl != null && cover == null) {
        try {
          final resp = await http.get(Uri.parse(_coverUrl!));
          if (resp.statusCode == 200) {
            cover = resp.bodyBytes;
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
      if (_coverRemoved) {
        cover = null;
      }
      // Genres are displayed joined with ", ", so they split on any of
      // ",;/"; the other multi-value fields are displayed joined with "; ".
      final genres = _ctrl('genres').text
          .split(RegExp(r'[,;/]'))
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      List<String> multi(String key) => _ctrl(key).text
          .split(';')
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toList();
      String? textOrNull(String key) =>
          _ctrl(key).text.isNotEmpty ? _ctrl(key).text : null;

      final data = EditableTagData(
        title: _ctrl('title').text,
        artist: _ctrl('artist').text,
        artists: multi('artists'),
        album: _ctrl('album').text,
        albumArtist: _ctrl('album_artist').text,
        albumArtists: multi('album_artists'),
        genres: genres,
        trackNumber: int.tryParse(_ctrl('track').text) ?? 0,
        totalTracks: int.tryParse(_ctrl('total_tracks').text) ?? 0,
        discNumber: int.tryParse(_ctrl('disc').text) ?? 0,
        totalDiscs: int.tryParse(_ctrl('total_discs').text) ?? 0,
        releaseDate: textOrNull('date'),
        originalDate: textOrNull('original_date'),
        media: _ctrl('media').text,
        script: _ctrl('script').text,
        recordLabel: _ctrl('record_label').text,
        releaseCountry: _ctrl('release_country').text,
        releaseType: multi('release_type'),
        isrc: _ctrl('isrc').text,
        mbidRecording: _ctrl('mbid_recording').text,
        mbidTrack: _ctrl('mbid_track').text,
        mbidArtist: _ctrl('mbid_artist').text,
        mbidRelease: _ctrl('mbid_release').text,
        mbidReleaseArtist: _ctrl('mbid_release_artist').text,
        mbidReleaseGroup: _ctrl('mbid_release_group').text,
        artistSort: _ctrl('artist_sort').text,
        albumArtistSort: _ctrl('album_artist_sort').text,
        comment: textOrNull('comment'),
        copyright: textOrNull('copyright'),
        lyrics: textOrNull('lyrics'),
        cover: cover,
        clearCover: _coverRemoved,
      );

      if (_fileBytes != null) {
        final result = await BridgeService.instance.writeFileTagsBytes(
          filename: _filename!,
          bytes: _fileBytes!,
          data: data,
        );
        if (!mounted) return;
        if (result == null) {
          AppSnackBar.show(
            context,
            'Tag editor is not available on this platform.',
            type: SnackType.error,
          );
        } else if (result.success && result.bytes != null) {
          await IOServiceFactory.create().downloadBytes(
            _filename!,
            result.bytes!,
          );
          if (!mounted) return;
          AppSnackBar.show(
            context,
            'Tags applied. The edited file is being downloaded.',
            type: SnackType.success,
          );
          Navigator.of(context).pop();
        } else {
          AppSnackBar.show(
            context,
            result.error ?? 'Failed to apply tags.',
            type: SnackType.error,
          );
        }
      } else {
        final result = await BridgeService.instance.writeFileTags(
          path: _path!,
          data: data,
        );
        if (!mounted) return;
        if (result == null) {
          AppSnackBar.show(
            context,
            'Tag editor is not available on this platform.',
            type: SnackType.error,
          );
        } else if (result.success) {
          AppSnackBar.show(
            context,
            'Tags applied successfully.',
            type: SnackType.success,
          );
          Navigator.of(context).pop();
        } else {
          AppSnackBar.show(
            context,
            result.error ?? 'Failed to apply tags.',
            type: SnackType.error,
          );
        }
      }
    } finally {
      if (mounted) setState(() => _applying = false);
    }
  }

  Widget _buildFileName() {
    final name = _filename ?? (_path != null ? basename(_path!) : null);
    if (name == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(bottom: AppTheme.spaceSM),
      child: Row(
        children: [
          const Icon(Icons.audiotrack, size: AppTheme.iconSM),
          const SizedBox(width: AppTheme.spaceXS),
          Expanded(
            child: Text(
              name,
              style: Theme.of(context).textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.manage_search),
            tooltip: 'Fetch from MusicBrainz',
            onPressed: _fetchFromMusicBrainz,
          ),
        ],
      ),
    );
  }

  Widget _buildField(
    String key,
    String label, {
    TextInputType? keyboardType,
    int? maxLines,
  }) {
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: maxLines != null && maxLines > 1
            ? CrossAxisAlignment.start
            : CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: _labelWidth,
            child: Text(
              label,
              style: textTheme.labelSmall?.copyWith(
                fontWeight: FontWeight.w600,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: TextField(
              controller: _ctrl(key),
              style: textTheme.bodySmall,
              keyboardType: keyboardType,
              maxLines: maxLines,
              minLines: maxLines != null ? 1 : 1,
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
        ],
      ),
    );
  }

  Widget _buildReadOnlyField(String label, String? value) {
    final textTheme = Theme.of(context).textTheme;
    final display = (value == null || value.isEmpty) ? '—' : value;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: _labelWidth,
            child: Text(
              label,
              style: textTheme.labelSmall?.copyWith(
                fontWeight: FontWeight.w600,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(
              display,
              style: textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Icon(
            Icons.lock_outline,
            size: AppTheme.iconSM,
            color: Theme.of(context).colorScheme.outline,
          ),
        ],
      ),
    );
  }

  Widget _section(String title, List<Widget> children) {
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(top: AppTheme.spaceMD),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: AppTheme.spaceXS),
          ...children,
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final isDesktop = AppTheme.isDesktop(context);
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    final dialogWidth = isDesktop
        ? AppTheme.dialogWidthDesktop
        : AppTheme.dialogWidthMobile;

    final fileName = _filename ?? (_path != null ? basename(_path!) : null);
    final header = fileName == null ? 'Tag Editor' : 'Edit Tags — $fileName';

    return AlertDialog(
      titlePadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceLG,
        AppTheme.spaceLG,
        0,
      ),
      title: Row(
        children: [
          Expanded(child: Text(header, style: textTheme.titleMedium)),
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
        width: dialogWidth,
        child: _buildBody(colors, textTheme),
      ),
      actionsPadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceSM,
        AppTheme.spaceLG,
        AppTheme.spaceLG,
      ),
      actions: [
        if (_tags != null)
          SizedBox(
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
      ],
    );
  }

  Widget _buildBody(ColorScheme colors, TextTheme textTheme) {
    if (_path == null && _filename == null) {
      return _buildFilePicker();
    }
    if (_loading) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(32),
          child: CircularProgressIndicator(),
        ),
      );
    }
    if (_error != null) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.error_outline, size: 48, color: colors.error),
          const SizedBox(height: AppTheme.spaceSM),
          Text(
            _error!,
            style: textTheme.bodyMedium?.copyWith(color: colors.error),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: AppTheme.spaceLG),
          OutlinedButton.icon(
            onPressed: () {
              setState(() {
                _path = null;
                _filename = null;
                _fileBytes = null;
                _error = null;
                _tags = null;
              });
            },
            icon: const Icon(Icons.refresh),
            label: const Text('Try another file'),
          ),
        ],
      );
    }
    if (_tags == null) {
      return const SizedBox.shrink();
    }

    final tags = _tags!;
    final filePath = _path ?? _filename;
    final trackGain = tags.trackGain == null
        ? null
        : '${tags.trackGain!.toStringAsFixed(2)} dB';

    return SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _buildFileName(),
          EditableCover(
            coverBytes:
                _selectedCover ??
                (tags.cover != null ? Uint8List.fromList(tags.cover!) : null),
            coverUrl: _coverUrl,
            replaced:
                _selectedCover != null || _coverUrl != null || _coverRemoved,
            onPickCover: _pickCover,
            onRevertCover: () => setState(() {
              _selectedCover = null;
              _coverUrl = null;
              _coverRemoved = false;
            }),
            onRemoveCover: () => setState(() {
              _selectedCover = null;
              _coverUrl = null;
              _coverRemoved = true;
            }),
            onPickCoverFromUrl: _pickCoverFromUrl,
          ),
          _section('Basic', [
            _buildField('title', 'Title'),
            _buildField('artist', 'Artist'),
            _buildField('artists', 'Artists'),
            _buildField('album', 'Album'),
            _buildField('album_artist', 'Album Artist'),
            _buildField('album_artists', 'Album Artists'),
            _buildField('genres', 'Genre(s)'),
          ]),
          _section('Track / Release', [
            _buildField('track', 'Track #', keyboardType: TextInputType.number),
            _buildField(
              'total_tracks',
              'Total Tracks',
              keyboardType: TextInputType.number,
            ),
            _buildField('disc', 'Disc #', keyboardType: TextInputType.number),
            _buildField(
              'total_discs',
              'Total Discs',
              keyboardType: TextInputType.number,
            ),
            _buildField('date', 'Date'),
            _buildField('original_date', 'Original Date'),
            _buildField('media', 'Media'),
            _buildField('script', 'Script'),
          ]),
          _section('Release / Identifiers', [
            _buildField('record_label', 'Record Label'),
            _buildField('release_country', 'Release Country'),
            _buildField('release_type', 'Release Type'),
            _buildField('isrc', 'ISRC'),
            _buildField('mbid_recording', 'Recording MBID'),
            _buildField('mbid_track', 'Track MBID'),
            _buildField('mbid_artist', 'Artist MBID'),
            _buildField('mbid_release_artist', 'Album Artist MBID'),
            _buildField('mbid_release', 'Release MBID'),
            _buildField('mbid_release_group', 'Release Group MBID'),
          ]),
          _section('Sorting', [
            _buildField('artist_sort', 'Artist Sort'),
            _buildField('album_artist_sort', 'Album Artist Sort'),
          ]),
          _section('ReplayGain', [
            _buildReadOnlyField('Track Gain', trackGain),
            _buildReadOnlyField('Track Peak', tags.trackPeak?.toString()),
          ]),
          _section('Lyrics / Comment / Copyright', [
            _buildField('lyrics', 'Lyrics', maxLines: 5),
            _buildField('comment', 'Comment', maxLines: 2),
            _buildField('copyright', 'Copyright'),
          ]),
          _section('Technical', [
            _buildReadOnlyField('File Path', filePath),
            _buildReadOnlyField('Length', formatDuration(tags.durationSecs)),
            _buildReadOnlyField('Encoder', tags.encoder),
          ]),
          const SizedBox(height: AppTheme.spaceSM),
        ],
      ),
    );
  }

  Widget _buildFilePicker() {
    return MediaUploader(
      allowedExtensions: audioBareExtensions,
      limit: 1,
      hintText: 'Select an audio file to edit its tags.',
      pickLabel: 'Select Audio File',
      onUpload: _handleUpload,
    );
  }
}
