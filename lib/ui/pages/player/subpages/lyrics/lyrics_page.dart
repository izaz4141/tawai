import 'package:flutter/material.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/pages/player/widgets/lyrics_view.dart';

class LyricsPage extends StatelessWidget {
  const LyricsPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: ValueListenableBuilder(
          valueListenable: PlaybackService.instance.queue,
          builder: (context, q, _) => Text(q.currentTrack?.title ?? 'Lyrics'),
        ),
        centerTitle: true,
      ),
      body: const LyricsView(),
    );
  }
}
