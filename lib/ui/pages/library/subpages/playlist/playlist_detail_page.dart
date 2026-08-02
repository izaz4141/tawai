import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/pages/library/subpages/playlist/playlist_header.dart';
import 'package:tawai/ui/widgets/components/track_list_tile.dart';
import 'package:tawai/utils/bridge_service.dart';

class PlaylistDetailPage extends StatefulWidget {
  const PlaylistDetailPage({super.key, required this.playlist});

  final PlaylistInfo playlist;

  @override
  State<PlaylistDetailPage> createState() => _PlaylistDetailPageState();
}

class _PlaylistDetailPageState extends State<PlaylistDetailPage> {
  List<TrackInfo> _tracks = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final tracks =
        await BridgeService.instance.getPlaylistTracks(widget.playlist.id);
    if (!mounted) return;
    setState(() {
      _tracks = tracks;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.playlist.name),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView.builder(
                physics: const AlwaysScrollableScrollPhysics(),
                itemCount: _tracks.length + 1,
                itemBuilder: (context, index) {
                  if (index == 0) {
                    return Column(
                      children: [
                        PlaylistHeader(playlist: widget.playlist),
                        if (_tracks.isEmpty)
                          const Padding(
                            padding: EdgeInsets.all(32),
                            child: Text('No tracks in this playlist'),
                          ),
                      ],
                    );
                  }
                  final track = _tracks[index - 1];
                  return TrackListTile(
                    track: track,
                    onTap: () => PlaybackService.instance.play(
                      _tracks,
                      startIndex: index - 1,
                    ),
                  );
                },
              ),
            ),
    );
  }
}
