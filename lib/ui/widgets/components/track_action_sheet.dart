import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/ui/pages/library/subpages/album/album_detail_page.dart';
import 'package:tawai/ui/pages/library/subpages/artist/artist_detail_page.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/download_track_sheet.dart';
import 'package:tawai/ui/widgets/components/track_info_sheet.dart';
import 'package:tawai/ui/widgets/dialog/tag_editor.dart';
import 'package:tawai/models/recommendation_source.dart';

void showTrackActionSheet(BuildContext context, TrackInfo track) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _TrackActionSheet(track: track),
  );
}

class _TrackActionSheet extends StatelessWidget {
  const _TrackActionSheet({required this.track});

  final TrackInfo track;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
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
                child: CoverImage(trackId: track.id, iconSize: AppTheme.iconMD),
              ),
            ),
            const SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(track.title, style: theme.textTheme.titleMedium),
                  if (track.artistsString.isNotEmpty ||
                      track.albumTitle.isNotEmpty)
                    Text(
                      '${track.artistsString}${track.albumTitle.isNotEmpty ? ' · ${track.albumTitle}' : ''}',
                      style: theme.textTheme.bodySmall,
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
            icon: Icons.play_arrow,
            label: 'Play',
            onTap: () {
              Navigator.pop(context);
              PlaybackService.instance.play([track]);
            },
          ),
          SheetActionItem(
            icon: Icons.skip_next,
            label: 'Play Next',
            onTap: () {
              Navigator.pop(context);
              PlaybackService.instance.queueNext(track);
            },
          ),
          SheetActionItem(
            icon: Icons.queue_music,
            label: 'Add to Queue',
            onTap: () {
              Navigator.pop(context);
              PlaybackService.instance.queueLast(track);
            },
          ),
          SheetActionItem(
            icon: Icons.playlist_add,
            label: 'Add to Playlist',
            onTap: () {
              Navigator.pop(context);
              _showAddToPlaylistDialog(context, track);
            },
          ),
          if (RecommendationSource.isRecommendationSource(track.sourceType))
            SheetActionItem(
              icon: Icons.download_for_offline,
              label: 'Download',
              onTap: () {
                Navigator.pop(context);
                showTrackDownloadSheet(context, track);
              },
            ),
          if (track.albumId.isNotEmpty)
            SheetActionItem(
              icon: Icons.album,
              label: 'View Album',
              onTap: () {
                Navigator.pop(context);
                Navigator.push(
                  context,
                  MaterialPageRoute(
                    builder: (_) => AlbumDetailPage(
                      album: AlbumInfo(
                        id: track.albumId,
                        title: track.albumTitle,
                        artistsString: track.artistsString,
                        artists: track.artists,
                        trackCount: 0,
                      ),
                    ),
                  ),
                );
              },
            ),
          if (track.artists.isNotEmpty)
            SheetActionItem(
              icon: Icons.person,
              label: 'View Artist',
              onTap: () {
                Navigator.pop(context);
                Navigator.push(
                  context,
                  MaterialPageRoute(
                    builder: (_) => ArtistDetailPage(
                      artist: ArtistInfo(
                        id: track.artists.first.id,
                        name: track.artistsString,
                        albumCount: 0,
                        trackCount: 0,
                      ),
                    ),
                  ),
                );
              },
            ),
          SheetActionItem(
            icon: Icons.edit,
            label: 'Edit Tags',
            onTap: () {
              showTagEditorDialog(context, initialPath: track.filePath);
            },
          ),
          SheetActionItem(
            icon: Icons.info_outline,
            label: 'Details',
            onTap: () => showTrackInfoSheet(context, track),
          ),
          if (!RecommendationSource.isRecommendationSource(track.sourceType))
            SheetActionItem(
              icon: Icons.delete_outline,
              label: 'Delete Track',
              onTap: () async {
                final confirmed = await showDialog<bool>(
                  context: context,
                  builder: (ctx) => AlertDialog(
                    title: const Text('Delete Track'),
                    content: Text('Delete "${track.title}" from your library?'),
                    actions: [
                      TextButton(
                        onPressed: () => Navigator.pop(ctx, false),
                        child: const Text('Cancel'),
                      ),
                      FilledButton(
                        onPressed: () => Navigator.pop(ctx, true),
                        child: const Text('Delete'),
                      ),
                    ],
                  ),
                );
                if (confirmed != true || !context.mounted) return;

                final result = await BridgeService.instance.deleteTrack(
                  track.id,
                );
                if (!context.mounted) return;

                AppSnackBar.show(
                  context,
                  result.success
                      ? 'Track deleted'
                      : 'Failed to delete: ${result.error ?? 'Unknown error'}',
                  type: result.success ? SnackType.success : SnackType.error,
                );
                Navigator.pop(context);
              },
            ),
        ],
      ),
    );
  }
}

Future<void> _showAddToPlaylistDialog(
  BuildContext context,
  TrackInfo track,
) async {
  final playlists = await BridgeService.instance.getPlaylists();
  if (!context.mounted) return;

  final selected = await showDialog<String>(
    context: context,
    builder: (ctx) => SimpleDialog(
      title: const Text('Add to Playlist'),
      children: playlists.map((p) {
        return SimpleDialogOption(
          onPressed: () => Navigator.pop(ctx, p.id),
          child: Text(p.name),
        );
      }).toList(),
    ),
  );

  if (selected == null || !context.mounted) return;

  final ok = await BridgeService.instance.addTrackToPlaylist(
    selected,
    track.id,
  );
  if (!context.mounted) return;

  AppSnackBar.show(
    context,
    ok ? 'Added to playlist' : 'Failed to add',
    type: ok ? SnackType.success : SnackType.error,
  );
}
