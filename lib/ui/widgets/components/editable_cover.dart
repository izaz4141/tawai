import 'dart:typed_data';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class EditableCover extends StatefulWidget {
  final Uint8List? coverBytes;
  final bool replaced;
  final VoidCallback onPickCover;
  final VoidCallback? onRevertCover;
  final double width;
  final double height;

  const EditableCover({
    super.key,
    required this.coverBytes,
    this.replaced = false,
    required this.onPickCover,
    this.onRevertCover,
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
    final hasArt = widget.coverBytes != null && widget.coverBytes!.isNotEmpty;

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
                child: hasArt
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
                      size: 32,
                      color: Colors.white,
                    ),
                  ),
                ),
              ),
              if (widget.replaced)
                Positioned(
                  top: 2,
                  right: 2,
                  child: Material(
                    color: colors.surfaceContainerHighest.withAlpha(200),
                    borderRadius: BorderRadius.circular(12),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(12),
                      onTap: widget.onRevertCover,
                      child: Padding(
                        padding: const EdgeInsets.all(2),
                        child: Icon(
                          Icons.undo,
                          size: 16,
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
      child: Icon(Icons.album, size: 48, color: colors.onSurfaceVariant),
    );
  }
}
