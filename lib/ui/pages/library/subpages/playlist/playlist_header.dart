import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class PlaylistHeader extends StatelessWidget {
  const PlaylistHeader({super.key, required this.playlist});

  final PlaylistInfo playlist;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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
            CircleAvatar(
              radius: AppTheme.radiusLG * AppTheme.radiusScale(context),
              backgroundColor: theme.colorScheme.primaryContainer,
              child: Icon(
                playlist.isSmart ? Icons.auto_awesome : Icons.playlist_play,
                size: AppTheme.iconLG * AppTheme.iconScale(context),
                color: theme.colorScheme.onPrimaryContainer,
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
                    playlist.name,
                    style: theme.textTheme.titleLarge,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (playlist.description != null &&
                      playlist.description!.isNotEmpty)
                    Padding(
                      padding: EdgeInsets.only(
                        top: AppTheme.spaceXS * AppTheme.spaceScale(context),
                      ),
                      child: Text(
                        playlist.description!,
                        style: theme.textTheme.bodySmall,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  SizedBox(
                    height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                  ),
                  Text(
                    '${playlist.trackCount} tracks${playlist.isSmart ? ' · Smart' : ''}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
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
