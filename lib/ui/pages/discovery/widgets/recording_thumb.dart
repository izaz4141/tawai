import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class RecordingThumb extends StatelessWidget {
  const RecordingThumb({super.key, required this.recording});

  final DiscoveryRecording recording;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    if (recording.cover != null) {
      return Image.network(
        recording.cover!,
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) => _fallback(colors),
      );
    }
    return _fallback(colors);
  }

  Widget _fallback(ColorScheme colors) {
    return Container(
      color: colors.surfaceContainerHighest,
      child: Icon(
        Icons.music_note,
        size: AppTheme.iconMD,
        color: colors.onSurfaceVariant,
      ),
    );
  }
}
