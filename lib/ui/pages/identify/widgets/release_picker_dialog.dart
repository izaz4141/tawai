import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cached_network_image.dart';

void showReleasePickerDialog({
  required BuildContext context,
  required RecordingInfo recording,
  required ValueChanged<ReleaseInfo> onSelected,
}) {
  showDialog(
    context: context,
    builder: (_) => SimpleDialog(
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
                  borderRadius: BorderRadius.circular(
                    AppTheme.radiusSM * AppTheme.radiusScale(context),
                  ),
                  child: CachedNetworkImage(
                    url:
                        'https://coverartarchive.org/release/${release.id}/front-250.jpg',
                    width: AppTheme.iconXL * AppTheme.iconScale(context),
                    height: AppTheme.iconXL * AppTheme.iconScale(context),
                    fit: BoxFit.cover,
                    placeholder: Container(
                      width: AppTheme.iconXL * AppTheme.iconScale(context),
                      height: AppTheme.iconXL * AppTheme.iconScale(context),
                      color: Theme.of(
                        context,
                      ).colorScheme.surfaceContainerHighest,
                      child: Icon(
                        Icons.album,
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ),
                SizedBox(
                  width: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                ),
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
                            release.country,
                            release.date,
                          ].whereType<String>().join(' · '),
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(
                                color: Theme.of(
                                  context,
                                ).colorScheme.onSurfaceVariant,
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
    ),
  );
}
