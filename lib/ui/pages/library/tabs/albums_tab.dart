import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/library/subpages/album/album_detail_page.dart';
import 'package:tawai/ui/pages/library/modal/album_action_sheet.dart';
import 'package:tawai/ui/widgets/components/alphabet_index_scroller.dart';
import 'package:tawai/ui/widgets/components/album_card.dart';
import 'package:tawai/ui/widgets/mini_player.dart';

class LibraryAlbumsTab extends StatelessWidget {
  const LibraryAlbumsTab({
    super.key,
    required this.albums,
    required this.scrollController,
  });

  final List<AlbumInfo> albums;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    if (albums.isEmpty) {
      return ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        children: [
          SizedBox(
            height: AppTheme.spaceMD * 10 * AppTheme.spaceScale(context),
          ),
          const Center(child: Text('No albums in library')),
        ],
      );
    }

    final isDesktop = AppTheme.isDesktop(context);
    final crossAxisCount = isDesktop ? 4 : 2;

    return ValueListenableBuilder<double>(
      valueListenable: miniPlayerInset,
      builder: (context, inset, _) {
        return Theme(
          data: Theme.of(context).copyWith(
            scrollbarTheme: const ScrollbarThemeData(
              thumbVisibility: WidgetStatePropertyAll(false),
              thickness: WidgetStatePropertyAll(0.0),
            ),
          ),
          child: GridView.builder(
            controller: scrollController,
            physics: const AlwaysScrollableScrollPhysics(),
            padding: EdgeInsets.fromLTRB(
              AppTheme.spaceSM * AppTheme.spaceScale(context),
              AppTheme.spaceSM * AppTheme.spaceScale(context),
              AppTheme.spaceSM * AppTheme.spaceScale(context) +
                  AlphabetIndexScroller.kStripWidth,
              AppTheme.spaceSM * AppTheme.spaceScale(context) + inset,
            ),
            gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: crossAxisCount,
              childAspectRatio: 0.85,
              crossAxisSpacing: AppTheme.spaceSM * AppTheme.spaceScale(context),
              mainAxisSpacing: AppTheme.spaceSM * AppTheme.spaceScale(context),
            ),
            itemCount: albums.length,
            itemBuilder: (context, index) {
              final a = albums[index];
              return AlbumCard(
                album: a,
                onTap: () => Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => AlbumDetailPage(album: a)),
                ),
                onLongPress: () => showAlbumActionSheet(context, a),
              );
            },
          ),
        );
      },
    );
  }
}
