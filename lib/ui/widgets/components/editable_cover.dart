import 'dart:typed_data';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

/// Prompts the user for an image URL. Returns the validated `http(s)` URL or
/// `null` if cancelled.
Future<String?> showCoverUrlDialog(BuildContext context) {
  return showDialog<String>(
    context: context,
    builder: (_) => const _CoverUrlDialog(),
  );
}

class _CoverUrlDialog extends StatefulWidget {
  const _CoverUrlDialog();

  @override
  State<_CoverUrlDialog> createState() => _CoverUrlDialogState();
}

class _CoverUrlDialogState extends State<_CoverUrlDialog> {
  final TextEditingController _urlCtrl = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _urlCtrl.dispose();
    super.dispose();
  }

  void _submit() {
    final url = _urlCtrl.text.trim();
    final uri = Uri.tryParse(url);
    if (uri == null || !(uri.isScheme('http') || uri.isScheme('https'))) {
      setState(() => _error = 'Enter a valid http(s) image URL.');
      return;
    }
    Navigator.of(context).pop(url);
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return AlertDialog(
      title: const Text('Add Cover from URL'),
      content: SizedBox(
        width: AppTheme.dialogWidthMobile,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _urlCtrl,
              autofocus: true,
              keyboardType: TextInputType.url,
              decoration: InputDecoration(
                isDense: true,
                hintText: 'https://example.com/cover.jpg',
                errorText: _error,
                errorMaxLines: 2,
              ),
              onSubmitted: (_) => _submit(),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(onPressed: _submit, child: const Text('Preview')),
      ],
    );
  }
}

class EditableCover extends StatefulWidget {
  final Uint8List? coverBytes;
  final String? coverUrl;
  final bool replaced;
  final VoidCallback onPickCover;
  final VoidCallback? onRevertCover;
  final VoidCallback? onPickCoverFromUrl;
  final double width;
  final double height;

  const EditableCover({
    super.key,
    required this.coverBytes,
    this.coverUrl,
    this.replaced = false,
    required this.onPickCover,
    this.onRevertCover,
    this.onPickCoverFromUrl,
    this.width = 120,
    this.height = 120,
  });

  @override
  State<EditableCover> createState() => _EditableCoverState();
}

class _EditableCoverState extends State<EditableCover> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final hasBytes = widget.coverBytes != null && widget.coverBytes!.isNotEmpty;
    final hasUrl = widget.coverUrl != null && widget.coverUrl!.isNotEmpty;
    final hasArt = hasBytes || hasUrl;

    return Center(
      child: SizedBox(
        width: widget.width,
        height: widget.height,
        child: InkWell(
          onTap: widget.onPickCover,
          onHover: (val) => setState(() => _hovered = val),
          borderRadius: BorderRadius.circular(AppTheme.radiusSM),
          child: Stack(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                child: hasUrl
                    ? Image.network(
                        widget.coverUrl!,
                        width: widget.width,
                        height: widget.height,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) => _placeholder(colors),
                      )
                    : hasBytes
                    ? Image.memory(
                        Uint8List.fromList(widget.coverBytes!),
                        width: widget.width,
                        height: widget.height,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) => _placeholder(colors),
                      )
                    : _placeholder(colors),
              ),
              AnimatedOpacity(
                opacity: _hovered ? 1.0 : 0.0,
                duration: const Duration(milliseconds: 150),
                curve: Curves.easeInOut,
                child: Container(
                  decoration: BoxDecoration(
                    color: Colors.black.withAlpha(80),
                    borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                  ),
                  child: Center(
                    child: Icon(
                      hasArt ? Icons.camera_alt : Icons.add_photo_alternate,
                      size: AppTheme.iconLG,
                      color: Colors.white,
                    ),
                  ),
                ),
              ),
              if (widget.onPickCoverFromUrl != null)
                Positioned(
                  top: AppTheme.spaceXS * 0.5,
                  left: AppTheme.spaceXS * 0.5,
                  child: Material(
                    color: colors.surfaceContainerHighest.withAlpha(200),
                    borderRadius: BorderRadius.circular(AppTheme.spaceXS * 3),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(AppTheme.spaceXS * 3),
                      onTap: widget.onPickCoverFromUrl,
                      child: Padding(
                        padding: const EdgeInsets.all(AppTheme.spaceXS * 0.5),
                        child: Icon(
                          Icons.link,
                          size: AppTheme.iconSM,
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ),
                ),
              if (widget.replaced && widget.onRevertCover != null)
                Positioned(
                  top: AppTheme.spaceXS * 0.5,
                  right: AppTheme.spaceXS * 0.5,
                  child: Material(
                    color: colors.surfaceContainerHighest.withAlpha(200),
                    borderRadius: BorderRadius.circular(AppTheme.spaceXS * 3),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(AppTheme.spaceXS * 3),
                      onTap: widget.onRevertCover,
                      child: Padding(
                        padding: const EdgeInsets.all(AppTheme.spaceXS * 0.5),
                        child: Icon(
                          Icons.undo,
                          size: AppTheme.iconSM,
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _placeholder(ColorScheme colors) {
    return Container(
      width: widget.width,
      height: widget.height,
      color: colors.surfaceContainerHighest,
      child: Icon(
        Icons.album,
        size: AppTheme.iconXXL,
        color: colors.onSurfaceVariant,
      ),
    );
  }
}
