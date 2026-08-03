import 'package:flutter/material.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/download_track_sheet.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_thumb.dart';

void showRecordingDetailsSheet(
  BuildContext context,
  DiscoveryRecording recording,
) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _RecordingDetailsSheet(recording: recording),
  );
}

class _RecordingDetailsSheet extends StatelessWidget {
  const _RecordingDetailsSheet({required this.recording});

  final DiscoveryRecording recording;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Sheet(
      showDivider: false,
      header: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(
              AppTheme.spaceXL,
              AppTheme.spaceMD,
              AppTheme.spaceXL,
              0,
            ),
            child: Row(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(AppTheme.radiusMD),
                  child: SizedBox(
                    width: AppTheme.iconXL * 3,
                    height: AppTheme.iconXL * 3,
                    child: RecordingThumb(recording: recording),
                  ),
                ),
                const SizedBox(width: AppTheme.spaceLG),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        recording.title,
                        style: textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: AppTheme.spaceXS),
                      Text(recording.artist, style: textTheme.bodyMedium),
                      if (recording.albumTitle != null) ...[
                        const SizedBox(height: AppTheme.spaceXS),
                        Text(
                          recording.albumTitle!,
                          style: textTheme.bodySmall?.copyWith(
                            color: colors.onSurfaceVariant,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(
              AppTheme.spaceXL,
              AppTheme.spaceMD,
              AppTheme.spaceXL,
              0,
            ),
            child: Wrap(
              spacing: AppTheme.spaceMD,
              runSpacing: AppTheme.spaceXS,
              children: [
                _MetaChip(
                  icon: Icons.timer_outlined,
                  label: recording.durationSecs != null
                      ? _fmtDuration(recording.durationSecs!)
                      : 'Unknown length',
                ),
                _MetaChip(
                  icon: Icons.star_outline,
                  label: recording.score.toStringAsFixed(2),
                ),
                if (recording.isOwned)
                  _MetaChip(
                    icon: Icons.check_circle,
                    label: 'In library',
                    foregroundColor: colors.onPrimaryContainer,
                    backgroundColor: colors.primaryContainer,
                  ),
              ],
            ),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceXL,
          AppTheme.spaceLG,
          AppTheme.spaceXL,
          AppTheme.spaceLG,
        ),
        child: Row(
          children: [
            Expanded(
              child: FilledButton.icon(
                icon: const Icon(Icons.play_arrow),
                label: const Text('Preview'),
                onPressed: () {
                  Navigator.pop(context);
                  PlaybackService.instance.preview(recording);
                },
              ),
            ),
            if (!recording.isOwned) ...[
              const SizedBox(width: AppTheme.spaceMD),
              Expanded(
                child: FilledButton.tonalIcon(
                  icon: const Icon(Icons.download),
                  label: const Text('Download'),
                  onPressed: () {
                    Navigator.pop(context);
                    showTrackDownloadSheet(
                      context,
                      PlaybackService.trackFromRecording(recording),
                    );
                  },
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _fmtDuration(double secs) {
    final total = secs.round();
    final m = total ~/ 60;
    final s = total % 60;
    return '${m.toString().padLeft(2, '0')}:${s.toString().padLeft(2, '0')}';
  }
}

class _MetaChip extends StatelessWidget {
  const _MetaChip({
    required this.icon,
    required this.label,
    this.foregroundColor,
    this.backgroundColor,
  });

  final IconData icon;
  final String label;
  final Color? foregroundColor;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final foreground = foregroundColor ?? colors.onSurfaceVariant;
    final background = backgroundColor ?? colors.surfaceContainerHighest;
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppTheme.spaceSM,
        vertical: AppTheme.spaceXS,
      ),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(AppTheme.radiusSM),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: AppTheme.iconSM, color: foreground),
          const SizedBox(width: AppTheme.spaceXS),
          Text(label, style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: foreground,
          )),
        ],
      ),
    );
  }
}
