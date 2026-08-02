import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';

class AlbumCard extends StatefulWidget {
  const AlbumCard({
    super.key,
    required this.album,
    required this.onTap,
    this.onLongPress,
  });

  final AlbumInfo album;
  final VoidCallback onTap;
  final VoidCallback? onLongPress;

  @override
  State<AlbumCard> createState() => _AlbumCardState();
}

class _AlbumCardState extends State<AlbumCard> {
  bool _isLoading = false;

  Future<void> _playAlbum() async {
    if (_isLoading) return;
    setState(() => _isLoading = true);
    try {
      final tracks = await BridgeService.instance
          .getTracks(albumId: widget.album.id);
      if (mounted) {
        await PlaybackService.instance.play(tracks);
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: GestureDetector(
        onSecondaryTap: AppTheme.isDesktop(context)
            ? widget.onLongPress
            : null,
        child: InkWell(
          onTap: widget.onTap,
          onLongPress: widget.onLongPress,
          child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Container(
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest,
                  borderRadius: const BorderRadius.vertical(
                      top: Radius.circular(12)),
                ),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    ClipRRect(
                      borderRadius: const BorderRadius.vertical(
                          top: Radius.circular(12)),
                      child: CoverImage(
                        albumId: widget.album.id,
                        fit: BoxFit.cover,
                        width: double.infinity,
                      ),
                    ),
                    Positioned(
                      right: 8,
                      bottom: 8,
                      child: Material(
                        type: MaterialType.transparency,
                        child: InkWell(
                          borderRadius: BorderRadius.circular(20),
                          onTap: _playAlbum,
                          child: Container(
                            width: 40,
                            height: 40,
                            decoration: BoxDecoration(
                              color: theme.colorScheme.primary,
                              shape: BoxShape.circle,
                            ),
                            child: Center(
                              child: _isLoading
                                  ? SizedBox(
                                      width: 16,
                                      height: 16,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                        color:
                                            theme.colorScheme.onPrimary,
                                      ),
                                    )
                                  : Icon(
                                      Icons.play_arrow_rounded,
                                      color: theme.colorScheme.onPrimary,
                                      size: 24,
                                    ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.album.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleSmall,
                  ),
                  Text(
                    widget.album.artistsString,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
      ),
    );
  }
}
