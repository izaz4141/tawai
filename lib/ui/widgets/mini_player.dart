import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/player/player_page.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/widgets/components/playback_button.dart';
import 'package:tawai/ui/widgets/components/volume_button.dart';

class MiniPlayer extends StatefulWidget {
  const MiniPlayer({super.key});

  @override
  State<MiniPlayer> createState() => _MiniPlayerState();
}

class _MiniPlayerState extends State<MiniPlayer> {
  double _hDrag = 0;
  double _vDrag = 0;

  void _onHorizontalDragUpdate(DragUpdateDetails d) {
    _hDrag = (_hDrag + d.delta.dx).clamp(-300, 300);
    setState(() {});
  }

  void _onHorizontalDragEnd(DragEndDetails d) {
    const threshold = 100.0;
    if (_hDrag.abs() > threshold) {
      final ps = PlaybackService.instance;
      if (_hDrag.isNegative) {
        ps.playNext();
      } else {
        ps.playPrevious();
      }
    }
    _hDrag = 0;
    setState(() {});
  }

  void _onVerticalDragUpdate(DragUpdateDetails d) {
    _vDrag = (_vDrag + d.delta.dy).clamp(-300, 300);
    setState(() {});
  }

  void _onVerticalDragEnd(DragEndDetails d) {
    if (_vDrag < -30) {
      Navigator.push(
        context,
        MaterialPageRoute(builder: (_) => const PlayerPage()),
      );
    } else if (_vDrag > 30) {
      PlaybackService.instance.clearQueue();
    }
    _vDrag = 0;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final size = MediaQuery.of(context).size;
    final hFrac = _hDrag / size.width;
    final vFrac = _vDrag / size.height;

    final ps = PlaybackService.instance;
    return ValueListenableBuilder<PlayerState>(
      valueListenable: ps.playerState,
      builder: (context, state, _) {
        final isLoading = state.processingState == ProcessingState.loading;
        return ValueListenableBuilder(
          valueListenable: ps.queue,
          builder: (context, q, _) {
            final track = q.currentTrack;
            if (track == null) {
              if (isLoading) {
                return Container(
                  height: 80,
                  margin: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(16),
                  ),
                  child: const Center(
                    child: SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(strokeWidth: 2.5),
                    ),
                  ),
                );
              }
              return const SizedBox.shrink();
            }

            return AnimatedSlide(
              offset: Offset(hFrac, vFrac),
              duration: (_hDrag == 0 && _vDrag == 0)
                  ? const Duration(milliseconds: 200)
                  : Duration.zero,
              curve: Curves.easeOut,
              child: Opacity(
                opacity:
                    (1 - vFrac.abs()).clamp(0.4, 1.0) *
                    (1 - hFrac.abs()).clamp(0.6, 1.0),
                child: GestureDetector(
                  onTap: () => Navigator.push(
                    context,
                    MaterialPageRoute(builder: (_) => const PlayerPage()),
                  ),
                  onHorizontalDragUpdate: _onHorizontalDragUpdate,
                  onHorizontalDragEnd: _onHorizontalDragEnd,
                  onVerticalDragUpdate: _onVerticalDragUpdate,
                  onVerticalDragEnd: _onVerticalDragEnd,
                  child: Container(
                    height: 80,
                    margin: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: colors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(16),
                      boxShadow: [
                        BoxShadow(
                          color: colors.shadow.withAlpha(60),
                          blurRadius: 12,
                          offset: const Offset(0, -2),
                        ),
                      ],
                    ),
                    clipBehavior: Clip.antiAlias,
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        _CoverWithPlayPause(track: track),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Padding(
                            padding: const EdgeInsets.only(right: 12),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                _TopRow(track: track),
                                const SizedBox(height: 2),
                                Row(
                                  children: [
                                    Expanded(child: _SeekRow()),
                                    _DurationLabel(),
                                  ],
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }
}

class _CoverWithPlayPause extends StatelessWidget {
  final TrackInfo track;
  const _CoverWithPlayPause({required this.track});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: AppTheme.spaceXXL * 2.5 * AppTheme.spaceScale(context),
      height: AppTheme.spaceXXL * 2.5 * AppTheme.spaceScale(context),
      child: Stack(
        children: [
          CoverImage(
            trackId: track.id,
            width: AppTheme.spaceXXL * 2.5 * AppTheme.spaceScale(context),
            height: AppTheme.spaceXXL * 2.5 * AppTheme.spaceScale(context),
            iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
          ),
          const Positioned.fill(child: _PlayOverlay()),
        ],
      ),
    );
  }
}

class _PlayOverlay extends StatelessWidget {
  const _PlayOverlay();

  @override
  Widget build(BuildContext context) {
    return Container(
      alignment: Alignment.center,
      decoration: BoxDecoration(color: Colors.black.withAlpha(80)),
      child: const PlaybackButton(iconSize: 32, color: Colors.white),
    );
  }
}

class _TopRow extends StatelessWidget {
  final TrackInfo track;
  const _TopRow({required this.track});

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                track.title,
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              Text(
                track.artistsString,
                style: Theme.of(context).textTheme.bodySmall,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
        ValueListenableBuilder(
          valueListenable: ps.shuffleModeEnabled,
          builder: (context, shuffle, _) {
            return IconButton(
              icon: Icon(
                Icons.shuffle,
                color: shuffle ? Theme.of(context).colorScheme.primary : null,
              ),
              iconSize: 18,
              visualDensity: VisualDensity.compact,
              onPressed: () => ps.toggleShuffle(),
            );
          },
        ),
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
              icon: Icon(icon, color: color),
              iconSize: 18,
              visualDensity: VisualDensity.compact,
              onPressed: ps.cycleLoopMode,
            );
          },
        ),
        const VolumeButton(iconSize: 18, popupWidth: 40, popupHeight: 124),
      ],
    );
  }
}

class _SeekRow extends StatefulWidget {
  @override
  State<_SeekRow> createState() => _SeekRowState();
}

class _SeekRowState extends State<_SeekRow> {
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
                trackHeight: 2,
                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 4),
                overlayShape: const RoundSliderOverlayShape(overlayRadius: 8),
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

class _DurationLabel extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    return ValueListenableBuilder<Duration>(
      valueListenable: ps.position,
      builder: (context, pos, _) {
        return ValueListenableBuilder<Duration>(
          valueListenable: ps.duration,
          builder: (context, dur, _) {
            return Padding(
              padding: const EdgeInsets.only(left: 4),
              child: Text(
                '${_fmt(pos)} / ${_fmt(dur)}',
                style: Theme.of(context).textTheme.labelSmall,
              ),
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
