import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';

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
                  borderRadius: BorderRadius.circular(4),
                  child: Image.network(
                    'https://coverartarchive.org/release/${release.id}/front-250.jpg',
                    width: 48,
                    height: 48,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => Container(
                      width: 48,
                      height: 48,
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
                const SizedBox(width: 16),
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
