import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class ReleasePickerDialog extends StatelessWidget {
  final RecordingInfo recording;
  final ValueChanged<ReleaseInfo> onSelected;

  const ReleasePickerDialog({
    super.key,
    required this.recording,
    required this.onSelected,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return SimpleDialog(
      title: Text(
        'Choose release',
        style: Theme.of(context).textTheme.titleMedium,
      ),
      children: recording.releases.map((release) {
        return SimpleDialogOption(
          onPressed: () {
            Navigator.of(context).pop();
            onSelected(release);
          },
          child: SizedBox(
            width: double.infinity,
            child: Row(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(AppTheme.radiusSM / 2),
                  child: Image.network(
                    'https://coverartarchive.org/release/${release.id}/front-250.jpg',
                    width: AppTheme.iconXL,
                    height: AppTheme.iconXL,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => Container(
                      width: AppTheme.iconXL,
                      height: AppTheme.iconXL,
                      color: colors.surfaceContainerHighest,
                      child: Icon(
                        Icons.album,
                        size: AppTheme.iconMD,
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: AppTheme.spaceMD),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        release.title,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      if (release.date != null || release.country != null)
                        Text(
                          [
                            if (release.country != null) release.country!,
                            if (release.date != null) release.date!,
                          ].join(' · '),
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: colors.onSurfaceVariant),
                        ),
                      if (release.totalTracks != null)
                        Padding(
                          padding: const EdgeInsets.only(top: AppTheme.spaceXS),
                          child: Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: AppTheme.spaceXS,
                              vertical: AppTheme.spaceXS / 2,
                            ),
                            decoration: BoxDecoration(
                              color: colors.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(
                                AppTheme.radiusSM,
                              ),
                            ),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  Icons.music_note,
                                  size: AppTheme.iconXS,
                                  color: colors.onSurfaceVariant,
                                ),
                                const SizedBox(width: AppTheme.spaceXS / 2),
                                Text(
                                  '${release.totalTracks} tracks',
                                  style: Theme.of(context).textTheme.labelSmall
                                      ?.copyWith(
                                        color: colors.onSurfaceVariant,
                                      ),
                                ),
                              ],
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
      }).toList(),
    );
  }
}

void showReleasePicker({
  required BuildContext context,
  required RecordingInfo recording,
  required ValueChanged<ReleaseInfo> onSelected,
}) {
  showDialog(
    context: context,
    builder: (_) =>
        ReleasePickerDialog(recording: recording, onSelected: onSelected),
  );
}
