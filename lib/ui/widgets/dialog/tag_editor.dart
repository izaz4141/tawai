import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/editable_cover.dart';
import 'package:tawai/ui/widgets/dialog/identify_dialog.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/io_service.dart';

const _audioExtensions = [
  'mp3',
  'flac',
  'ogg',
  'm4a',
  'mp4',
  'wav',
  'aiff',
  'aac',
  'ape',
  'mpc',
  'wv',
  'opus',
  'spx',
];

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
  String? _path;
  bool _loading = false;
  bool _applying = false;
  String? _error;
  ReadFileTagsResponse? _tags;
  Uint8List? _selectedCover;

  late final TextEditingController _titleCtrl;
  late final TextEditingController _artistCtrl;
  late final TextEditingController _albumCtrl;
  late final TextEditingController _albumArtistCtrl;
  late final TextEditingController _genreCtrl;
  late final TextEditingController _trackCtrl;
  late final TextEditingController _discCtrl;
  late final TextEditingController _yearCtrl;
  late final TextEditingController _lyricsCtrl;

  @override
  void initState() {
    super.initState();
    _titleCtrl = TextEditingController();
    _artistCtrl = TextEditingController();
    _albumCtrl = TextEditingController();
    _albumArtistCtrl = TextEditingController();
    _genreCtrl = TextEditingController();
    _trackCtrl = TextEditingController();
    _discCtrl = TextEditingController();
    _yearCtrl = TextEditingController();
    _lyricsCtrl = TextEditingController();

    if (widget.initialPath != null) {
      _path = widget.initialPath;
      _loadTags();
    }
  }

  @override
  void dispose() {
    _titleCtrl.dispose();
    _artistCtrl.dispose();
    _albumCtrl.dispose();
    _albumArtistCtrl.dispose();
    _genreCtrl.dispose();
    _trackCtrl.dispose();
    _discCtrl.dispose();
    _yearCtrl.dispose();
    _lyricsCtrl.dispose();
    super.dispose();
  }

  Future<void> _pickFile() async {
    final result = await IOServiceFactory.create().pickFile(
      allowedExtensions: _audioExtensions,
    );
    final path = result?.path;
    if (path == null) return;
    setState(() => _path = path);
    _loadTags();
  }

  Future<void> _pickCover() async {
    final result = await IOServiceFactory.create().pickFile(
      allowedExtensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'],
    );
    final bytes = result?.bytes;
    if (bytes == null) return;
    setState(() => _selectedCover = bytes);
  }

  Future<void> _fetchFromMusicBrainz() async {
    final result = await showIdentifyDialog(
      context,
      currentTitle: _titleCtrl.text,
      currentArtist: _artistCtrl.text,
      currentAlbum: _albumCtrl.text,
    );
    if (result == null || !mounted) return;
    setState(() {
      _titleCtrl.text = result.title;
      _artistCtrl.text = result.artist;
      _albumCtrl.text = result.album;
      _albumArtistCtrl.text = result.artist;
      _trackCtrl.text = result.trackNumber?.toString() ?? '';
      _discCtrl.text = result.discNumber?.toString() ?? '';
      _yearCtrl.text = result.releaseDate ?? '';
      if (result.coverBytes != null) {
        _selectedCover = result.coverBytes;
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
    try {
      final response = await BridgeService.instance.readFileTags(_path!);
      if (!mounted) return;
      if (response == null) {
        setState(() {
          _error = 'Tag editor is not available on this platform.';
          _loading = false;
        });
        return;
      }
      if (response.error != null) {
        setState(() {
          _error = response.error;
          _loading = false;
        });
        return;
      }
      setState(() {
        _tags = response;
        _selectedCover = null;
        _loading = false;
      });
      _titleCtrl.text = response.title;
      _artistCtrl.text = response.artist;
      _albumCtrl.text = response.album;
      _albumArtistCtrl.text = response.albumArtist;
      _genreCtrl.text = response.genres.join(', ');
      _trackCtrl.text = response.trackNumber > 0
          ? response.trackNumber.toString()
          : '';
      _discCtrl.text = response.discNumber > 0
          ? response.discNumber.toString()
          : '';
      _yearCtrl.text = response.releaseDate ?? '';
      _lyricsCtrl.text = response.lyrics ?? '';
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _apply() async {
    if (_path == null || _tags == null) return;
    setState(() => _applying = true);
    try {
      final result = await BridgeService.instance.writeFileTags(
        path: _path!,
        title: _titleCtrl.text,
        artist: _artistCtrl.text,
        album: _albumCtrl.text,
        albumArtist: _albumArtistCtrl.text,
        genres: _genreCtrl.text
            .split(RegExp(r'[,;/]'))
            .map((s) => s.trim())
            .where((s) => s.isNotEmpty)
            .toList(),
        trackNumber: int.tryParse(_trackCtrl.text) ?? 0,
        discNumber: int.tryParse(_discCtrl.text) ?? 0,
        releaseDate: _yearCtrl.text.isNotEmpty ? _yearCtrl.text : null,
        lyrics: _lyricsCtrl.text.isNotEmpty ? _lyricsCtrl.text : null,
        cover: _selectedCover,
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
    } finally {
      if (mounted) setState(() => _applying = false);
    }
  }

  Widget _buildFileName() {
    if (_path == null) return const SizedBox.shrink();
    final name = _path!.split(Platform.pathSeparator).last;
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
    String label,
    TextEditingController controller, {
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
            width: 100,
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
              controller: controller,
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

  @override
  Widget build(BuildContext context) {
    final isDesktop = AppTheme.isDesktop(context);
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    final dialogWidth = isDesktop
        ? AppTheme.dialogWidthDesktop
        : AppTheme.dialogWidthMobile;

    final header = _path == null
        ? 'Tag Editor'
        : 'Edit Tags — ${_path!.split(Platform.pathSeparator).last}';

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
    if (_path == null) {
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

    return SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          _buildFileName(),
          EditableCover(
            coverBytes:
                _selectedCover ??
                (_tags?.cover != null
                    ? Uint8List.fromList(_tags!.cover!)
                    : null),
            replaced: _selectedCover != null,
            onPickCover: _pickCover,
            onRevertCover: () => setState(() => _selectedCover = null),
          ),
          _buildField('Title', _titleCtrl),
          _buildField('Artist', _artistCtrl),
          _buildField('Album', _albumCtrl),
          _buildField('Album Artist', _albumArtistCtrl),
          _buildField('Genre(s)', _genreCtrl),
          _buildField(
            'Track #',
            _trackCtrl,
            keyboardType: TextInputType.number,
          ),
          _buildField('Disc #', _discCtrl, keyboardType: TextInputType.number),
          _buildField('Year', _yearCtrl),
          _buildField('Lyrics', _lyricsCtrl, maxLines: 5),
          const SizedBox(height: AppTheme.spaceSM),
        ],
      ),
    );
  }

  Widget _buildFilePicker() {
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.audiotrack,
            size: 64,
            color: Theme.of(context).colorScheme.primary,
          ),
          const SizedBox(height: AppTheme.spaceMD),
          Text(
            'Select an audio file to edit its tags.',
            style: textTheme.bodyMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: AppTheme.spaceLG),
          FilledButton.icon(
            onPressed: _pickFile,
            icon: const Icon(Icons.file_open),
            label: const Text('Select Audio File'),
          ),
        ],
      ),
    );
  }
}
