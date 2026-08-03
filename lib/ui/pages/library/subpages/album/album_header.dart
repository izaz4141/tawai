import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
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
      margin: const EdgeInsets.all(16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: SizedBox(
                width: 120,
                height: 120,
                child: CoverImage(albumId: album.id),
              ),
            ),
            const SizedBox(width: 16),
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
                  const SizedBox(height: 4),
                  Text(
                    album.artistsString,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.primary,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 4),
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
