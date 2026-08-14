import 'dart:math';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/pages/library/subpages/album/album_header.dart';
import 'package:tawai/ui/widgets/components/track_list_tile.dart';
import 'package:tawai/ui/widgets/mini_player.dart';

class AlbumDetailPage extends StatefulWidget {
  const AlbumDetailPage({super.key, required this.album});

  final AlbumInfo album;

  @override
  State<AlbumDetailPage> createState() => _AlbumDetailPageState();
}

class _AlbumDetailPageState extends State<AlbumDetailPage> {
  List<TrackInfo> _tracks = [];
  bool _loading = true;
  bool _isPlayingAll = false;

  @override
  void initState() {
    super.initState();
    _loadTracks();
    BridgeService.libraryChanged.addListener(_onLibraryChanged);
  }

  @override
  void dispose() {
    BridgeService.libraryChanged.removeListener(_onLibraryChanged);
    super.dispose();
  }

  void _onLibraryChanged() {
    if (mounted) _loadTracks();
  }

  Future<void> _loadTracks() async {
    final tracks = await BridgeService.instance.getTracks(
      albumId: widget.album.id,
    );
    if (mounted) {
      setState(() {
        _tracks = tracks;
        _loading = false;
      });
    }
  }

  Future<void> _playAll() async {
    if (_isPlayingAll || _tracks.isEmpty) return;
    setState(() => _isPlayingAll = true);
    try {
      await PlaybackService.instance.play(_tracks);
    } finally {
      if (mounted) setState(() => _isPlayingAll = false);
    }
  }

  Future<void> _shufflePlay() async {
    if (_isPlayingAll || _tracks.isEmpty) return;
    setState(() => _isPlayingAll = true);
    try {
      final shuffled = List<TrackInfo>.from(_tracks)..shuffle(Random());
      await PlaybackService.instance.play(shuffled);
    } finally {
      if (mounted) setState(() => _isPlayingAll = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(widget.album.title)),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _loadTracks,
              child: ListView.builder(
                physics: const AlwaysScrollableScrollPhysics(),
                itemCount: _tracks.length + 3,
                itemBuilder: (context, index) {
                  if (index == _tracks.length + 2) {
                    return const MiniPlayerSpacer();
                  }
                  if (index == 0) {
                    return AlbumHeader(
                      album: widget.album,
                      trackCount: _tracks.length,
                    );
                  }
                  if (index == 1) {
                    return Card(
                      margin: EdgeInsets.fromLTRB(
                        AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        0,
                        AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
                        AppTheme.spaceSM * AppTheme.spaceScale(context),
                      ),
                      child: Padding(
                        padding: EdgeInsets.symmetric(
                          horizontal:
                              AppTheme.spaceSM * AppTheme.spaceScale(context),
                          vertical:
                              AppTheme.spaceXS * AppTheme.spaceScale(context),
                        ),
                        child: Row(
                          children: [
                            Expanded(
                              child: TextButton.icon(
                                onPressed: _isPlayingAll ? null : _playAll,
                                icon: _isPlayingAll
                                    ? SizedBox(
                                        width:
                                            AppTheme.spaceLG *
                                            AppTheme.spaceScale(context),
                                        height:
                                            AppTheme.spaceLG *
                                            AppTheme.spaceScale(context),
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                          color: theme.colorScheme.primary,
                                        ),
                                      )
                                    : const Icon(Icons.play_arrow_rounded),
                                label: const Text('Play All'),
                              ),
                            ),
                            Expanded(
                              child: TextButton.icon(
                                onPressed: _isPlayingAll ? null : _shufflePlay,
                                icon: const Icon(Icons.shuffle),
                                label: const Text('Shuffle'),
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  }
                  final track = _tracks[index - 2];
                  return TrackListTile(
                    track: track,
                    index: index - 2,
                    onTap: () => PlaybackService.instance.play(
                      _tracks,
                      startIndex: index - 2,
                    ),
                  );
                },
              ),
            ),
    );
  }
}
