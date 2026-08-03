import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/library/subpages/playlist/playlist_detail_page.dart';
import 'package:tawai/ui/widgets/components/alphabet_index_scroller.dart';

class LibraryPlaylistsTab extends StatelessWidget {
  const LibraryPlaylistsTab({
    super.key,
    required this.playlists,
    required this.scrollController,
    required this.onCreatePlaylist,
    required this.onDeletePlaylist,
  });

  final List<PlaylistInfo> playlists;
  final ScrollController scrollController;
  final VoidCallback onCreatePlaylist;
  final void Function(String playlistId) onDeletePlaylist;

  @override
  Widget build(BuildContext context) {
    if (playlists.isEmpty) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          _buildHeader(context),
          const SizedBox(height: 80),
          const Center(child: Text('No playlists yet')),
        ],
      );
    }
    return Theme(
      data: Theme.of(context).copyWith(
        scrollbarTheme: const ScrollbarThemeData(
          thumbVisibility: WidgetStatePropertyAll(false),
          thickness: WidgetStatePropertyAll(0.0),
        ),
      ),
      child: ListView.builder(
        padding: EdgeInsets.only(right: AlphabetIndexScroller.kStripWidth),
        controller: scrollController,
        physics: const AlwaysScrollableScrollPhysics(),
        itemCount: playlists.length + 1,
        itemBuilder: (context, index) {
          if (index == 0) return _buildHeader(context);
          final p = playlists[index - 1];
          return ListTile(
            leading: Icon(p.isSmart ? Icons.auto_awesome : Icons.playlist_play),
            title: Text(p.name, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${p.trackCount} tracks${p.isSmart ? " · Smart" : ""}',
            ),
            trailing: IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: () => onDeletePlaylist(p.id),
            ),
            onTap: () => Navigator.push(
              context,
              MaterialPageRoute(
                builder: (_) => PlaylistDetailPage(playlist: p),
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _buildHeader(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: Row(
        children: [
          Expanded(
            child: Text(
              '${playlists.length} playlists',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ),
          FilledButton.icon(
            onPressed: onCreatePlaylist,
            icon: const Icon(Icons.add, size: 18),
            label: const Text('New'),
          ),
        ],
      ),
    );
  }
}
