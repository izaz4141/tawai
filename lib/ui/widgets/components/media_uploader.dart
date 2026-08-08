import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/utils/io_service.dart';

class UploadedMediaFile {
  final String name;
  final Uint8List? bytes;
  final String? path;

  const UploadedMediaFile({required this.name, this.bytes, this.path});

  factory UploadedMediaFile.fromPickFileResult(PickFileResult result) {
    return UploadedMediaFile(
      name: result.name ?? '',
      bytes: result.bytes,
      path: result.path,
    );
  }
}

class UploadMediaResult {
  final bool success;
  final String? error;

  const UploadMediaResult({required this.success, this.error});
}

enum _UploadStatus { pending, uploading, done, failed }

class _UploadEntry {
  final UploadedMediaFile file;
  _UploadStatus status;
  String? error;

  _UploadEntry(this.file) : status = _UploadStatus.pending;
}

class MediaUploader extends StatefulWidget {
  final List<String>? allowedExtensions;
  final int? limit;
  final String pickLabel;
  final String? hintText;
  final Future<UploadMediaResult> Function(UploadedMediaFile file) onUpload;
  final void Function(UploadedMediaFile file)? onUploaded;

  const MediaUploader({
    super.key,
    this.allowedExtensions,
    this.limit,
    this.pickLabel = 'Choose File',
    this.hintText,
    required this.onUpload,
    this.onUploaded,
  }) : assert(limit == null || limit > 0);

  @override
  State<MediaUploader> createState() => _MediaUploaderState();
}

class _MediaUploaderState extends State<MediaUploader> {
  final List<_UploadEntry> _entries = [];

  bool get _isFull => widget.limit != null && _entries.length >= widget.limit!;

  Future<void> _pickFiles() async {
    final result = await IOServiceFactory.create().pickFile(
      allowedExtensions: widget.allowedExtensions,
    );
    if (result == null) return;
    if (!mounted) return;
    if (widget.limit != null && (_entries.length + 1) > widget.limit!) {
      AppSnackBar.show(
        context,
        'Maximum ${widget.limit} file${widget.limit == 1 ? '' : 's'} allowed.',
        type: SnackType.error,
      );
      return;
    }
    _addFile(UploadedMediaFile.fromPickFileResult(result));
  }

  void _addFile(UploadedMediaFile file) {
    final entry = _UploadEntry(file);
    setState(() => _entries.add(entry));
    if (_isFull) {
      _upload(entry);
    }
  }

  void _removeEntry(_UploadEntry entry) {
    if (entry.status == _UploadStatus.uploading) return;
    setState(() => _entries.remove(entry));
  }

  Future<void> _upload(_UploadEntry entry) async {
    if (entry.status == _UploadStatus.uploading) return;
    setState(() {
      entry.status = _UploadStatus.uploading;
      entry.error = null;
    });
    try {
      final result = await widget.onUpload(entry.file);
      if (!mounted) return;
      if (result.success) {
        setState(() {
          entry.status = _UploadStatus.done;
        });
        widget.onUploaded?.call(entry.file);
      } else {
        setState(() {
          entry.status = _UploadStatus.failed;
          entry.error = result.error;
        });
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        entry.status = _UploadStatus.failed;
        entry.error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_entries.isEmpty) {
      return _buildEmpty();
    }
    if (_entries.any((e) => e.status == _UploadStatus.uploading)) {
      return _buildUploading();
    }
    return _buildList();
  }

  Widget _buildEmpty() {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.upload_file,
            size: AppTheme.iconXXL,
            color: colors.primary,
          ),
          SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
          Text(
            widget.hintText ?? 'Select media file(s) to upload.',
            style: textTheme.bodyMedium,
            textAlign: TextAlign.center,
          ),
          SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
          FilledButton.icon(
            onPressed: _pickFiles,
            icon: const Icon(Icons.file_open),
            label: Text(widget.pickLabel),
          ),
        ],
      ),
    );
  }

  Widget _buildUploading() {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final uploading = _entries
        .where((e) => e.status == _UploadStatus.uploading)
        .map((e) => e.file.name)
        .join(', ');
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: AppTheme.iconXXL,
            height: AppTheme.iconXXL,
            child: CircularProgressIndicator(
              strokeWidth: 3,
              color: colors.primary,
            ),
          ),
          SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
          Text('Uploading…', style: textTheme.bodyMedium),
          SizedBox(height: AppTheme.spaceXS * AppTheme.spaceScale(context)),
          Text(
            uploading,
            style: textTheme.bodySmall,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  Widget _buildList() {
    final textTheme = Theme.of(context).textTheme;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ..._entries.map(_buildEntry),
        if (!_isFull)
          Padding(
            padding: const EdgeInsets.only(top: AppTheme.spaceSM),
            child: OutlinedButton.icon(
              onPressed: _pickFiles,
              icon: Icon(
                Icons.add,
                size: AppTheme.iconMD * AppTheme.iconScale(context),
              ),
              label: Text(widget.pickLabel, style: textTheme.bodyMedium),
            ),
          ),
      ],
    );
  }

  Widget _buildEntry(_UploadEntry entry) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: AppTheme.spaceXS),
      child: Row(
        children: [
          Icon(Icons.audiotrack, size: AppTheme.iconSM, color: colors.primary),
          SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          Expanded(
            child: Text(
              entry.file.name,
              style: textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          _buildTrailing(entry, colors),
        ],
      ),
    );
  }

  Widget _buildTrailing(_UploadEntry entry, ColorScheme colors) {
    switch (entry.status) {
      case _UploadStatus.pending:
        return FilledButton.tonalIcon(
          onPressed: () => _upload(entry),
          icon: const Icon(Icons.cloud_upload, size: AppTheme.iconSM),
          label: const Text('Upload'),
        );
      case _UploadStatus.uploading:
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: AppTheme.iconSM,
              height: AppTheme.iconSM,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: colors.primary,
              ),
            ),
            SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            Text('Uploading…', style: Theme.of(context).textTheme.bodySmall),
          ],
        );
      case _UploadStatus.done:
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.check_circle,
              size: AppTheme.iconSM,
              color: Colors.green,
            ),
            SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            Text('Uploaded', style: Theme.of(context).textTheme.bodySmall),
            _buildRemoveButton(entry),
          ],
        );
      case _UploadStatus.failed:
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Tooltip(
              message: entry.error ?? 'Upload failed.',
              child: Icon(
                Icons.error,
                size: AppTheme.iconSM,
                color: colors.error,
              ),
            ),
            SizedBox(width: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            FilledButton.tonalIcon(
              onPressed: () => _upload(entry),
              icon: const Icon(Icons.refresh, size: AppTheme.iconSM),
              label: const Text('Retry'),
            ),
            _buildRemoveButton(entry),
          ],
        );
    }
  }

  Widget _buildRemoveButton(_UploadEntry entry) {
    final colors = Theme.of(context).colorScheme;
    return IconButton(
      icon: Icon(
        Icons.close,
        size: AppTheme.iconSM,
        color: colors.onSurfaceVariant,
      ),
      tooltip: 'Remove',
      onPressed: () => _removeEntry(entry),
    );
  }
}
