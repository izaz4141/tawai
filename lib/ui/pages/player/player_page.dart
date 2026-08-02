import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/models/recommendation_source.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/widgets/components/playback_button.dart';
import 'package:tawai/ui/widgets/components/volume_button.dart';
import 'package:tawai/ui/widgets/components/download_track_sheet.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/player/modal/queue_sheet.dart';
import 'package:tawai/ui/pages/lyrics/lyrics_page.dart';

class PlayerPage extends StatelessWidget {
  const PlayerPage({super.key});

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;

    return Scaffold(
      extendBodyBehindAppBar: true,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.keyboard_arrow_down_rounded),
          onPressed: () => Navigator.pop(context),
        ),
      ),
      body: ValueListenableBuilder<PlayerState>(
        valueListenable: PlaybackService.instance.playerState,
        builder: (context, state, _) {
          final isLoading = state.processingState == ProcessingState.loading;
          return ValueListenableBuilder(
            valueListenable: PlaybackService.instance.queue,
            builder: (context, q, _) {
              final track = q.currentTrack;
              if (track == null) {
                if (isLoading) {
                  return const Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SizedBox(
                          width: 32,
                          height: 32,
                          child: CircularProgressIndicator(strokeWidth: 3),
                        ),
                        SizedBox(height: 16),
                        Text('Loading track...'),
                      ],
                    ),
                  );
                }
                return const Center(child: Text('No track playing'));
              }

              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Flexible(
                    flex: 3,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: AppTheme.spaceXL * 2,
                        vertical: AppTheme.spaceXL,
                      ),
                      child: Center(
                        child: ConstrainedBox(
                          constraints: const BoxConstraints(
                            maxWidth: 400,
                            maxHeight: 400,
                          ),
                          child: AspectRatio(
                            aspectRatio: 1,
                            child: ClipRRect(
                              borderRadius: BorderRadius.circular(
                                AppTheme.radiusSM * 2,
                              ),
                              child: CoverImage(
                                trackId: track.id,
                                fit: BoxFit.contain,
                                iconSize: AppTheme.iconXXL,
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: AppTheme.spaceXL,
                    ),
                    child: Column(
                      children: [
                        Text(
                          track.title,
                          style: textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: AppTheme.spaceXS),
                        Text(
                          track.artistsString,
                          style: textTheme.bodyLarge,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                        ),
                      ],
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(
                      AppTheme.spaceXL,
                      AppTheme.spaceSM,
                      AppTheme.spaceXL,
                      0,
                    ),
                    child: Row(
                      children: [
                        Expanded(child: _PlayerSeekRow()),
                        const SizedBox(width: AppTheme.spaceSM),
                        _PlayerDurationLabel(),
                      ],
                    ),
                  ),
                  _PlayerControlsRow(),
                  SizedBox(
                    height: AppTheme.iconXL,
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        TextButton.icon(
                          icon: const Icon(Icons.queue_music),
                          label: const Text('Queue'),
                          onPressed: () => showModalBottomSheet(
                            context: context,
                            isScrollControlled: true,
                            builder: (_) => const QueueSheet(),
                          ),
                        ),
                        const SizedBox(width: AppTheme.spaceXXL),
                        TextButton.icon(
                          icon: const Icon(Icons.lyrics_outlined),
                          label: const Text('Lyrics'),
                          onPressed: () => Navigator.push(
                            context,
                            MaterialPageRoute(builder: (_) => const LyricsPage()),
                          ),
                        ),
                        if (RecommendationSource.isRecommendationSource(
                          track.sourceType,
                        )) ...[
                          const SizedBox(width: AppTheme.spaceXXL),
                          TextButton.icon(
                            icon: const Icon(Icons.download),
                            label: const Text('Download'),
                            onPressed: () => showTrackDownloadSheet(context, track),
                          ),
                        ],
                      ],
                    ),
                  ),
                  const Expanded(flex: 3, child: SizedBox.shrink()),
                ],
              );
            },
          );
        },
      ),
    );
  }
}

class _PlayerSeekRow extends StatefulWidget {
  @override
  State<_PlayerSeekRow> createState() => _PlayerSeekRowState();
}

class _PlayerSeekRowState extends State<_PlayerSeekRow> {
  bool _dragging = false;
  double _dragValue = 0;

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return ValueListenableBuilder<Duration>(
      valueListenable: ps.duration,
      builder: (context, dur, _) {
        return ValueListenableBuilder<Duration>(
          valueListenable: ps.position,
          builder: (context, pos, _) {
            final max = dur.inMilliseconds > 0 ? dur.inMilliseconds : 1.0;
            final value = _dragging
                ? _dragValue
                : (pos.inMilliseconds / max).clamp(0.0, 1.0);
            return SliderTheme(
              data: SliderTheme.of(context).copyWith(
                trackHeight: 4,
                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
              ),
              child: Material(
                type: MaterialType.transparency,
                child: Slider(
                  value: value,
                  onChanged: (v) {
                    _dragging = true;
                    _dragValue = v;
                  },
                  onChangeEnd: (v) {
                    _dragging = false;
                    ps.seek(Duration(milliseconds: (v * max).toInt()));
                  },
                ),
              ),
            );
          },
        );
      },
    );
  }
}

class _PlayerDurationLabel extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return ValueListenableBuilder<Duration>(
      valueListenable: ps.position,
      builder: (context, pos, _) {
        return ValueListenableBuilder<Duration>(
          valueListenable: ps.duration,
          builder: (context, dur, _) {
            return Text(
              '${_fmt(pos)} / ${_fmt(dur)}',
              style: Theme.of(context).textTheme.labelMedium,
            );
          },
        );
      },
    );
  }

  String _fmt(Duration d) {
    final min = d.inMinutes.remainder(60);
    final sec = d.inSeconds.remainder(60);
    return '${min.toString().padLeft(2, '0')}:${sec.toString().padLeft(2, '0')}';
  }
}

class _PlayerControlsRow extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return SizedBox(
      height: 80,
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          ValueListenableBuilder(
            valueListenable: ps.shuffleModeEnabled,
            builder: (context, shuffle, _) {
              return IconButton(
                icon: Icon(
                  Icons.shuffle,
                  size: AppTheme.iconMD,
                  color: shuffle ? Theme.of(context).colorScheme.primary : null,
                ),
                onPressed: () => ps.toggleShuffle(),
              );
            },
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          IconButton(
            icon: const Icon(
              Icons.skip_previous_rounded,
              size: AppTheme.iconLG + AppTheme.spaceXS,
            ),
            onPressed: () => ps.playPrevious(),
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          PlaybackButton(
            iconSize: AppTheme.iconXXL,
            color: Theme.of(context).colorScheme.primary,
            playIcon: Icons.play_circle_fill_rounded,
            pauseIcon: Icons.pause_circle_filled_rounded,
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          IconButton(
            icon: const Icon(
              Icons.skip_next_rounded,
              size: AppTheme.iconLG + AppTheme.spaceXS,
            ),
            onPressed: () => ps.playNext(),
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          ValueListenableBuilder(
            valueListenable: ps.loopMode,
            builder: (context, mode, _) {
              final colors = Theme.of(context).colorScheme;
              IconData icon;
              Color? color;
              switch (mode) {
                case LoopMode.one:
                  icon = Icons.repeat_one;
                  color = colors.primary;
                case LoopMode.all:
                  icon = Icons.repeat;
                  color = colors.primary;
                case LoopMode.off:
                  icon = Icons.repeat;
                  color = null;
              }
              return IconButton(
                icon: Icon(icon, size: AppTheme.iconMD, color: color),
                onPressed: ps.cycleLoopMode,
              );
            },
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          const VolumeButton(),
        ],
      ),
    );
  }
}
