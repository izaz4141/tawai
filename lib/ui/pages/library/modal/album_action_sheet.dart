import 'dart:math';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/ui/pages/library/subpages/artist/artist_detail_page.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';

void showAlbumActionSheet(BuildContext context, AlbumInfo album) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _AlbumActionSheet(album: album),
  );
}

class _AlbumActionSheet extends StatelessWidget {
  const _AlbumActionSheet({required this.album});

  final AlbumInfo album;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Sheet(
      header: Padding(
        padding: EdgeInsets.fromLTRB(
          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
          AppTheme.spaceMD * AppTheme.spaceScale(context),
          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
          AppTheme.spaceXS * AppTheme.spaceScale(context),
        ),
        child: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(
                AppTheme.radiusSM * AppTheme.radiusScale(context),
              ),
              child: SizedBox(
                width: AppTheme.spaceSM * 7 * AppTheme.spaceScale(context),
                height: AppTheme.spaceSM * 7 * AppTheme.spaceScale(context),
                child: CoverImage(
                  albumId: album.id,
                  iconSize: AppTheme.iconLG * AppTheme.iconScale(context),
                ),
              ),
            ),
            SizedBox(width: AppTheme.spaceMD * AppTheme.spaceScale(context)),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(album.title, style: theme.textTheme.titleMedium),
                  Text(
                    [
                      album.artistsString,
                      if (album.releaseDate != null) '${album.releaseDate}',
                      '${album.trackCount} tracks',
                    ].join(' · '),
                    style: theme.textTheme.bodySmall,
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
            icon: Icons.play_arrow,
            label: 'Play All',
            onTap: () async {
              Navigator.pop(context);
              final tracks = await BridgeService.instance.getTracks(
                albumId: album.id,
              );
              if (tracks.isNotEmpty) {
                await PlaybackService.instance.play(tracks);
              }
            },
          ),
          SheetActionItem(
            icon: Icons.shuffle,
            label: 'Shuffle Play',
            onTap: () async {
              Navigator.pop(context);
              final tracks = await BridgeService.instance.getTracks(
                albumId: album.id,
              );
              if (tracks.isNotEmpty) {
                tracks.shuffle(Random());
                await PlaybackService.instance.play(tracks);
              }
            },
          ),
          SheetActionItem(
            icon: Icons.queue_music,
            label: 'Add to Queue',
            onTap: () async {
              Navigator.pop(context);
              final tracks = await BridgeService.instance.getTracks(
                albumId: album.id,
              );
              for (final track in tracks) {
                PlaybackService.instance.queueLast(track);
              }
            },
          ),
          if (album.artists.isNotEmpty)
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
                        id: album.artists.first.id,
                        name: album.artistsString,
                        albumCount: 0,
                        trackCount: 0,
                      ),
                    ),
                  ),
                );
              },
            ),
        ],
      ),
    );
  }
}
