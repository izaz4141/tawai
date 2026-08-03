import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/widgets/components/track_action_sheet.dart';
import 'package:tawai/utils/helper.dart';

class LibraryHistoryTab extends StatelessWidget {
  const LibraryHistoryTab({
    super.key,
    required this.history,
    required this.onPlayRecord,
  });

  final List<PlaybackRecord> history;
  final void Function(PlaybackRecord record) onPlayRecord;

  @override
  Widget build(BuildContext context) {
    if (history.isEmpty) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: const [
          SizedBox(height: 120),
          Center(child: Text('No playback history yet')),
        ],
      );
    }
    return ListView.builder(
      physics: const AlwaysScrollableScrollPhysics(),
      itemCount: history.length,
      itemBuilder: (context, index) {
        final r = history[index];
        final isJellyfin = r.source == 'jellyfin';
        return ListTile(
          leading: CircleAvatar(
            child: Text(
              r.trackTitle.isNotEmpty ? r.trackTitle[0].toUpperCase() : '?',
            ),
          ),
          title: Text(
            r.trackTitle,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            '${r.artistName} · ${r.albumTitle} · ${formatTimestamp(r.playedAt)}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (isJellyfin)
                Tooltip(
                  message: 'From Jellyfin',
                  child: Icon(
                    Icons.cloud,
                    size: 16,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                )
              else if (r.scrobbled)
                Tooltip(
                  message: 'Scrobbled to ListenBrainz',
                  child: Icon(
                    Icons.check_circle_outline,
                    size: 16,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                ),
              const SizedBox(width: 8),
              Text(
                formatDuration(r.durationSecs),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
          onTap: () => onPlayRecord(r),
          onLongPress: () => _showActionSheetForRecord(context, r),
        );
      },
    );
  }

  void _showActionSheetForRecord(BuildContext context, PlaybackRecord record) {
    PlaybackService.instance.fetchTrackInfo(record.trackId).then((track) {
      if (track != null && context.mounted) {
        showTrackActionSheet(context, track);
      }
    });
  }
}
