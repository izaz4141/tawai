import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class TrackPickerDialog extends StatelessWidget {
  final ReleaseInfo release;
  final List<ReleaseTrackInfo> tracks;
  final ValueChanged<ReleaseTrackInfo> onSelected;

  const TrackPickerDialog({
    super.key,
    required this.release,
    required this.tracks,
    required this.onSelected,
  });

  String _formatDuration(double? secs) {
    if (secs == null) return '';
    final totalSecs = secs.round();
    final minutes = totalSecs ~/ 60;
    final seconds = totalSecs % 60;
    return '${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return SimpleDialog(
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Choose track', style: Theme.of(context).textTheme.titleMedium),
          Text(
            release.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
          ),
        ],
      ),
      children: tracks.map((track) {
        return SimpleDialogOption(
          onPressed: () {
            Navigator.of(context).pop();
            onSelected(track);
          },
          child: SizedBox(
            width: double.infinity,
            child: Row(
              children: [
                if (track.position != null)
                  SizedBox(
                    width: AppTheme.spaceXL + AppTheme.spaceXS,
                    child: Text(
                      '${track.position}',
                      style: Theme.of(
                        context,
                      ).textTheme.bodySmall?.copyWith(color: colors.outline),
                    ),
                  ),
                const SizedBox(width: AppTheme.spaceSM),
                Expanded(
                  child: Text(
                    track.title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (track.durationSecs != null)
                  Text(
                    _formatDuration(track.durationSecs),
                    style: Theme.of(
                      context,
                    ).textTheme.bodySmall?.copyWith(color: colors.outline),
                  ),
              ],
            ),
          ),
        );
      }).toList(),
    );
  }
}

void showTrackPicker({
  required BuildContext context,
  required ReleaseInfo release,
  required List<ReleaseTrackInfo> tracks,
  required ValueChanged<ReleaseTrackInfo> onSelected,
}) {
  showDialog(
    context: context,
    builder: (_) => TrackPickerDialog(
      release: release,
      tracks: tracks,
      onSelected: onSelected,
    ),
  );
}
