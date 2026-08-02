import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/library/subpages/artist/artist_detail_page.dart';
import 'package:tawai/ui/widgets/components/alphabet_index_scroller.dart';

class LibraryArtistsTab extends StatelessWidget {
  const LibraryArtistsTab({
    super.key,
    required this.artists,
    required this.scrollController,
  });

  final List<ArtistInfo> artists;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    if (artists.isEmpty) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: const [
          SizedBox(height: 120),
          Center(child: Text('No artists in library')),
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
        itemCount: artists.length,
        itemBuilder: (context, index) {
          final a = artists[index];
          return ListTile(
            leading: CircleAvatar(
              child: Text(a.name.isNotEmpty ? a.name[0].toUpperCase() : '?'),
            ),
            title: Text(a.name, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text('${a.albumCount} albums · ${a.trackCount} tracks'),
            onTap: () => Navigator.push(
              context,
              MaterialPageRoute(
                builder: (_) => ArtistDetailPage(artist: a),
              ),
            ),
          );
        },
      ),
    );
  }
}
