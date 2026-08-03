import 'package:flutter/material.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/download_track_sheet.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_details_sheet.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_thumb.dart';

void showRecordingActionSheet(
  BuildContext context,
  DiscoveryRecording recording,
) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _RecordingActionSheet(recording: recording),
  );
}

class _RecordingActionSheet extends StatelessWidget {
  const _RecordingActionSheet({required this.recording});

  final DiscoveryRecording recording;

  @override
  Widget build(BuildContext context) {
    return Sheet(
      header: Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceXL,
          AppTheme.spaceMD,
          AppTheme.spaceXL,
          AppTheme.spaceXS,
        ),
        child: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              child: SizedBox(
                width: AppTheme.iconXL,
                height: AppTheme.iconXL,
                child: RecordingThumb(recording: recording),
              ),
            ),
            const SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    recording.title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  Text(
                    recording.artist,
                    style: Theme.of(context).textTheme.bodySmall,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SheetActionItem(
            icon: Icons.info_outline,
            label: 'See Details',
            onTap: () {
              Navigator.pop(context);
              showRecordingDetailsSheet(context, recording);
            },
          ),
          if (!recording.isOwned)
            SheetActionItem(
              icon: Icons.download_for_offline,
              label: 'Download',
              onTap: () {
                Navigator.pop(context);
                showTrackDownloadSheet(
                  context,
                  PlaybackService.trackFromRecording(recording),
                );
              },
            ),
        ],
      ),
    );
  }
}
