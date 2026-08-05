import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/preview_button.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_action_sheet.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_thumb.dart';

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

    return GestureDetector(
      onLongPress: () => showRecordingActionSheet(context, recording),
      onSecondaryTap: AppTheme.isDesktop(context)
          ? () => showRecordingActionSheet(context, recording)
          : null,
      child: Card(
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
                    top: Radius.circular(
                      AppTheme.radiusMD * AppTheme.radiusScale(context),
                    ),
                  ),
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      RecordingThumb(recording: recording),
                      if (recording.isOwned)
                        Positioned(
                          top: AppTheme.spaceSM * scale,
                          right: AppTheme.spaceSM * scale,
                          child: Material(
                            color: colors.primary,
                            shape: const CircleBorder(),
                            child: Padding(
                              padding: EdgeInsets.all(AppTheme.spaceXS * scale),
                              child: Icon(
                                Icons.check_circle,
                                size:
                                    AppTheme.iconSM *
                                    AppTheme.iconScale(context),
                                color: colors.onPrimary,
                              ),
                            ),
                          ),
                        ),
                      Positioned(
                        right: AppTheme.spaceSM * scale,
                        bottom: AppTheme.spaceSM * scale,
                        child: PreviewButton(
                          recording: recording,
                          iconSize: AppTheme.iconMD,
                          backgroundColor: colors.primaryContainer.withValues(
                            alpha: 0.9,
                          ),
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
      ),
    );
  }
}
