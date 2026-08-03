import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:tawai/services/playback_service.dart';

class PlaybackButton extends StatelessWidget {
  final double iconSize;
  final Color? color;
  final IconData playIcon;
  final IconData pauseIcon;

  const PlaybackButton({
    super.key,
    this.iconSize = 48,
    this.color,
    this.playIcon = Icons.play_arrow_rounded,
    this.pauseIcon = Icons.pause_rounded,
  });

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<PlayerState>(
      valueListenable: PlaybackService.instance.playerState,
      builder: (context, state, _) {
        final isLoading =
            state.processingState == ProcessingState.loading ||
            state.processingState == ProcessingState.buffering;
        final touchTarget = iconSize < 48 ? 48.0 : iconSize;

        return Material(
          type: MaterialType.transparency,
          child: InkWell(
            borderRadius: BorderRadius.circular(touchTarget * 0.5),
            onTap: () => PlaybackService.instance.togglePlayPause(),
            child: SizedBox(
              width: touchTarget,
              height: touchTarget,
              child: Center(
                child: isLoading
                    ? SizedBox(
                        width: iconSize * 0.5,
                        height: iconSize * 0.5,
                        child: CircularProgressIndicator(
                          strokeWidth: 2.5,
                          color: color,
                        ),
                      )
                    : Icon(
                        state.playing ? pauseIcon : playIcon,
                        size: iconSize,
                        color: color,
                      ),
              ),
            ),
          ),
        );
      },
    );
  }
}
