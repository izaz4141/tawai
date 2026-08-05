import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';

class AlbumHeader extends StatelessWidget {
  const AlbumHeader({super.key, required this.album, this.trackCount});

  final AlbumInfo album;
  final int? trackCount;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final displayCount = trackCount ?? album.trackCount;

    return Card(
      margin: EdgeInsets.all(
        AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
      ),
      child: Padding(
        padding: EdgeInsets.all(
          AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
        ),
        child: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(
                AppTheme.radiusSM * AppTheme.radiusScale(context),
              ),
              child: SizedBox(
                width: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
                height: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
                child: CoverImage(albumId: album.id),
              ),
            ),
            SizedBox(
              width: AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    album.title,
                    style: theme.textTheme.titleLarge,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  SizedBox(
                    height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                  ),
                  Text(
                    album.artistsString,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.primary,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  SizedBox(
                    height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                  ),
                  Text(
                    [
                      if (album.releaseDate != null) '${album.releaseDate}',
                      '$displayCount tracks',
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
    );
  }
}
