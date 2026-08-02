import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/preview_button.dart';

class RecordingCard extends StatelessWidget {
  final DiscoveryRecording recording;

  const RecordingCard({super.key, required this.recording});

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final scale = AppTheme.spaceScale(context);
    final isDesktop = AppTheme.isDesktop(context);
    final cardWidth = isDesktop ? 160.0 : 140.0;

    return Card(
      margin: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceXS * scale,
        vertical: AppTheme.spaceXS * scale,
      ),
      child: SizedBox(
        width: cardWidth,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Expanded(
              child: ClipRRect(
                borderRadius: BorderRadius.vertical(
                  top: Radius.circular(AppTheme.radiusMD * AppTheme.radiusScale(context)),
                ),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    recording.cover != null
                        ? Image.network(
                            recording.cover!,
                            width: cardWidth,
                            fit: BoxFit.cover,
                            errorBuilder: (_, _, _) => _buildFallback(colors),
                          )
                        : _buildFallback(colors),
                    if (recording.isOwned)
                      Positioned(
                        top: 6,
                        right: 6,
                        child: Material(
                          color: colors.primary,
                          shape: const CircleBorder(),
                          child: Padding(
                            padding: EdgeInsets.all(2),
                            child: Icon(
                              Icons.check_circle,
                              size: 18,
                              color: colors.onPrimary,
                            ),
                          ),
                        ),
                      ),
                    Positioned(
                      right: 6,
                      bottom: 6,
                      child: PreviewButton(
                        recording: recording,
                        iconSize: AppTheme.iconMD,
                        backgroundColor: colors.primaryContainer.withOpacity(0.9),
                        foregroundColor: colors.onPrimaryContainer,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Padding(
              padding: EdgeInsets.all(AppTheme.spaceSM * scale),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Tooltip(
                    message: recording.title,
                    child: Text(
                      recording.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodySmall?.copyWith(
                        fontWeight: FontWeight.w600,
                        color: colors.onSurface,
                      ),
                    ),
                  ),
                  SizedBox(height: AppTheme.spaceXS * scale),
                  Tooltip(
                    message: recording.artist,
                    child: Text(
                      recording.artist,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFallback(ColorScheme colors) {
    return Container(
      width: double.infinity,
      color: colors.surfaceContainerHighest,
      child: Icon(
        Icons.music_note,
        size: AppTheme.iconMD,
        color: colors.onSurfaceVariant,
      ),
    );
  }
}
