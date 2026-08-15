import 'package:flutter/material.dart';
import 'package:tawai/ui/pages/identify/controller/identify_controller.dart';
import 'package:tawai/ui/pages/identify/widgets/album_result_card.dart';
import 'package:tawai/ui/widgets/mini_player.dart';

class RightPanel extends StatelessWidget {
  final IdentifyController controller;

  const RightPanel({super.key, required this.controller});

  @override
  Widget build(BuildContext context) {
    return ListView.separated(
      itemCount: controller.albumResults.length + 1,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        if (index == controller.albumResults.length) {
          return const MiniPlayerSpacer();
        }
        final entry = controller.albumResults.entries.elementAt(index);
        return AlbumResultCard(
          albumKey: entry.key,
          album: entry.value,
          controller: controller,
        );
      },
    );
  }
}
