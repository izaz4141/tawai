import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/widgets/components/track_action_sheet.dart';
import 'package:tawai/utils/helper.dart';

class TrackListTile extends StatelessWidget {
  const TrackListTile({super.key, required this.track, this.index, this.onTap});

  final TrackInfo track;
  final int? index;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onLongPress: () => showTrackActionSheet(context, track),
      onSecondaryTap: AppTheme.isDesktop(context)
          ? () => showTrackActionSheet(context, track)
          : null,
      child: ListTile(
        leading: ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: SizedBox(
            width: 40,
            height: 40,
            child: CoverImage(trackId: track.id, iconSize: 20),
          ),
        ),
        title: Text(track.title, maxLines: 1, overflow: TextOverflow.ellipsis),
        subtitle: Text(
          '${track.artistsString} · ${track.albumTitle}${track.durationSecs != null ? " · ${formatDuration(track.durationSecs)}" : ""}',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        onTap: onTap ?? () => PlaybackService.instance.play([track]),
      ),
    );
  }
}
