import 'dart:math';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/library/subpages/album/album_detail_page.dart';
import 'package:tawai/ui/pages/library/modal/album_action_sheet.dart';
import 'package:tawai/ui/pages/library/subpages/artist/artist_header.dart';
import 'package:tawai/ui/widgets/components/album_card.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/services/playback_service.dart';

class ArtistDetailPage extends StatefulWidget {
  const ArtistDetailPage({super.key, required this.artist});

  final ArtistInfo artist;

  @override
  State<ArtistDetailPage> createState() => _ArtistDetailPageState();
}

class _ArtistDetailPageState extends State<ArtistDetailPage> {
  List<AlbumInfo> _albums = [];
  bool _loading = true;
  bool _isPlayingAll = false;

  @override
  void initState() {
    super.initState();
    _loadAlbums();
  }

  Future<void> _loadAlbums() async {
    final albums = await BridgeService.instance
        .getAlbums(artistId: widget.artist.id);
    if (mounted) {
      setState(() {
        _albums = albums;
        _loading = false;
      });
    }
  }

  Future<void> _playAllArtist() async {
    if (_isPlayingAll) return;
    setState(() => _isPlayingAll = true);
    try {
      final allTracks = <TrackInfo>[];
      for (final album in _albums) {
        final tracks = await BridgeService.instance
            .getTracks(albumId: album.id);
        allTracks.addAll(tracks);
      }
      if (allTracks.isEmpty) return;
      if (mounted) {
        await PlaybackService.instance.play(allTracks);
      }
    } finally {
      if (mounted) setState(() => _isPlayingAll = false);
    }
  }

  Future<void> _shufflePlayArtist() async {
    if (_isPlayingAll) return;
    setState(() => _isPlayingAll = true);
    try {
      final allTracks = <TrackInfo>[];
      for (final album in _albums) {
        final tracks = await BridgeService.instance
            .getTracks(albumId: album.id);
        allTracks.addAll(tracks);
      }
      if (allTracks.isEmpty) return;
      allTracks.shuffle(Random());
      if (mounted) {
        await PlaybackService.instance.play(allTracks);
      }
    } finally {
      if (mounted) setState(() => _isPlayingAll = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isDesktop = AppTheme.isDesktop(context);
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.artist.name),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _loadAlbums,
              child: _albums.isEmpty
                  ? ListView(
                      physics: const AlwaysScrollableScrollPhysics(),
                      children: [
                        ArtistHeader(artist: widget.artist),
                        const Center(
                          child: Padding(
                            padding: EdgeInsets.all(32),
                            child: Text('No albums found'),
                          ),
                        ),
                      ],
                    )
                  : CustomScrollView(
                      physics: const AlwaysScrollableScrollPhysics(),
                      slivers: [
                        SliverToBoxAdapter(
                          child: ArtistHeader(artist: widget.artist),
                        ),
                        SliverToBoxAdapter(
                          child: Card(
                            margin: const EdgeInsets.fromLTRB(16, 0, 16, 8),
                            child: Padding(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                              child: Row(
                                children: [
                                  Expanded(
                                    child: TextButton.icon(
                                      onPressed: _isPlayingAll ? null : _playAllArtist,
                                      icon: _isPlayingAll
                                          ? SizedBox(
                                              width: 18,
                                              height: 18,
                                              child: CircularProgressIndicator(
                                                strokeWidth: 2,
                                                color: theme.colorScheme.primary,
                                              ),
                                            )
                                          : Icon(Icons.play_arrow_rounded),
                                      label: const Text('Play All'),
                                    ),
                                  ),
                                  Expanded(
                                    child: TextButton.icon(
                                      onPressed: _isPlayingAll ? null : _shufflePlayArtist,
                                      icon: const Icon(Icons.shuffle),
                                      label: const Text('Shuffle'),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                        SliverPadding(
                          padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                          sliver: SliverGrid(
                            gridDelegate:
                                SliverGridDelegateWithFixedCrossAxisCount(
                              crossAxisCount: isDesktop ? 4 : 2,
                              childAspectRatio: 0.85,
                              crossAxisSpacing: 8,
                              mainAxisSpacing: 8,
                            ),
                            delegate: SliverChildBuilderDelegate(
                              (context, index) => AlbumCard(
                                album: _albums[index],
                                onTap: () => Navigator.push(
                                  context,
                                  MaterialPageRoute(
                                    builder: (_) =>
                                        AlbumDetailPage(album: _albums[index]),
                                  ),
                                ),
                                onLongPress: () =>
                                    showAlbumActionSheet(context, _albums[index]),
                              ),
                              childCount: _albums.length,
                            ),
                          ),
                        ),
                      ],
                    ),
            ),
    );
  }
}


