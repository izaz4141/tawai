import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';

class ArtistHeader extends StatelessWidget {
  const ArtistHeader({super.key, required this.artist});

  final ArtistInfo artist;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.all(16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            CircleAvatar(
              radius: 48,
              backgroundColor: theme.colorScheme.surfaceContainerHighest,
              child: artist.thumbnailUrl != null
                  ? ClipRRect(
                      borderRadius: BorderRadius.circular(48),
                      child: Image.network(
                        artist.thumbnailUrl!,
                        width: 96,
                        height: 96,
                        fit: BoxFit.cover,
                        errorBuilder: (_, _, _) => _initial(theme),
                      ),
                    )
                  : _initial(theme),
            ),
            const SizedBox(width: 16),
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
                  const SizedBox(height: 4),
                  Text(
                    '${artist.albumCount} albums · ${artist.trackCount} tracks',
                    style: theme.textTheme.bodySmall,
                  ),
                  if (artist.sortName != null && artist.sortName != artist.name)
                    Padding(
                      padding: const EdgeInsets.only(top: 2),
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
