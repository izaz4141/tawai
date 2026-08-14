import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cached_network_image.dart';

class ArtistHeader extends StatelessWidget {
  const ArtistHeader({super.key, required this.artist});

  final ArtistInfo artist;

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
              radius: AppTheme.spaceMD * 4 * AppTheme.radiusScale(context),
              backgroundColor: theme.colorScheme.surfaceContainerHighest,
              child: artist.thumbnailUrl != null
                  ? ClipRRect(
                      borderRadius: BorderRadius.circular(
                        AppTheme.spaceMD * 4 * AppTheme.radiusScale(context),
                      ),
                      child: CachedNetworkImage(
                        url: artist.thumbnailUrl!,
                        width:
                            AppTheme.spaceMD * 8 * AppTheme.spaceScale(context),
                        height:
                            AppTheme.spaceMD * 8 * AppTheme.spaceScale(context),
                        fit: BoxFit.cover,
                        placeholder: _initial(theme),
                      ),
                    )
                  : _initial(theme),
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
                    artist.name,
                    style: theme.textTheme.titleLarge,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  SizedBox(
                    height: AppTheme.spaceXS * AppTheme.spaceScale(context),
                  ),
                  Text(
                    '${artist.albumCount} albums · ${artist.trackCount} tracks',
                    style: theme.textTheme.bodySmall,
                  ),
                  if (artist.sortName != null && artist.sortName != artist.name)
                    Padding(
                      padding: EdgeInsets.only(
                        top: AppTheme.spaceXS * AppTheme.spaceScale(context),
                      ),
                      child: Text(
                        'Sort by: ${artist.sortName}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
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

  Widget _initial(ThemeData theme) {
    return Text(
      artist.name.isNotEmpty ? artist.name[0].toUpperCase() : '?',
      style: theme.textTheme.headlineMedium?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
  }
}
