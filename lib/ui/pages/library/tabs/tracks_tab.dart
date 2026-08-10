import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/track_list_tile.dart';
import 'package:tawai/ui/widgets/components/alphabet_index_scroller.dart';
import 'package:tawai/ui/widgets/mini_player.dart';

class LibraryTracksTab extends StatelessWidget {
  const LibraryTracksTab({
    super.key,
    required this.tracks,
    required this.scrollController,
  });

  final List<TrackInfo> tracks;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    if (tracks.isEmpty) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          SizedBox(
            height: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
          ),
          const Center(child: Text('No tracks in library')),
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
        itemCount: tracks.length + 1,
        itemBuilder: (context, index) {
          if (index == tracks.length) return const MiniPlayerSpacer();
          return TrackListTile(track: tracks[index], index: index);
        },
      ),
    );
  }
}
