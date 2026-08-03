import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';

class PreviewButton extends StatelessWidget {
  final DiscoveryRecording recording;
  final double iconSize;
  final Color backgroundColor;
  final Color foregroundColor;

  const PreviewButton({
    super.key,
    required this.recording,
    this.iconSize = 24,
    required this.backgroundColor,
    required this.foregroundColor,
  });

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return Material(
      color: backgroundColor,
      shape: const CircleBorder(),
      child: InkWell(
        customBorder: const CircleBorder(),
        onTap: () => ps.preview(recording),
        child: Padding(
          padding: EdgeInsets.all(4),
          child: ValueListenableBuilder<PlayerState>(
            valueListenable: ps.playerState,
            builder: (context, state, _) {
              final isLoading =
                  state.processingState == ProcessingState.loading ||
                  state.processingState == ProcessingState.buffering;
              final isCurrentTrack = ps.currentTrack?.id == recording.id;
              final isActive = isCurrentTrack && state.playing;

              if (isLoading && isCurrentTrack) {
                return SizedBox(
                  width: iconSize,
                  height: iconSize,
                  child: Center(
                    child: SizedBox(
                      width: iconSize * 0.6,
                      height: iconSize * 0.6,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: foregroundColor,
                      ),
                    ),
                  ),
                );
              }

              return Icon(
                isActive ? Icons.pause_rounded : Icons.play_arrow_rounded,
                size: iconSize,
                color: foregroundColor,
              );
            },
          ),
        ),
      ),
    );
  }
}
