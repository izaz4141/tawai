import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/player/player_page.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/widgets/components/playback_button.dart';
import 'package:tawai/ui/widgets/components/volume_button.dart';
import 'package:tawai/ui/widgets/components/seek_bar.dart';

const double kMiniPlayerHeight = 96;
final ValueNotifier<double> miniPlayerInset = ValueNotifier(0);

class MiniPlayerSpacer extends StatelessWidget {
  const MiniPlayerSpacer({super.key});

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<double>(
      valueListenable: miniPlayerInset,
      builder: (context, inset, _) => SizedBox(height: inset),
    );
  }
}

class MiniPlayer extends StatefulWidget {
  const MiniPlayer({super.key});

  @override
  State<MiniPlayer> createState() => _MiniPlayerState();
}

class _MiniPlayerState extends State<MiniPlayer> {
  Offset _drag = Offset.zero;
  Axis? _dragAxis;

  void _syncInset(bool visible) {
    final value = visible ? kMiniPlayerHeight : 0.0;
    if (miniPlayerInset.value == value) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (miniPlayerInset.value != value) {
        miniPlayerInset.value = value;
      }
    });
  }

  void _onPanUpdate(DragUpdateDetails d) {
    _drag += d.delta;
    if (_dragAxis == null && (_drag.dx.abs() > 8 || _drag.dy.abs() > 8)) {
      _dragAxis = _drag.dx.abs() >= _drag.dy.abs()
          ? Axis.horizontal
          : Axis.vertical;
    }
    setState(() {});
  }

  void _onPanEnd(DragEndDetails d) {
    final axis = _dragAxis;
    if (axis == Axis.horizontal && _drag.dx.abs() > 100) {
      final ps = PlaybackService.instance;
      if (_drag.dx.isNegative) {
        ps.playNext();
      } else {
        ps.playPrevious();
      }
    } else if (axis == Axis.vertical) {
      if (_drag.dy < -30) {
        Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const PlayerPage()),
        );
      } else if (_drag.dy > 30) {
        PlaybackService.instance.clearQueue();
      }
    }
    _drag = Offset.zero;
    _dragAxis = null;
    setState(() {});
  }

  void _onPanCancel() {
    _drag = Offset.zero;
    _dragAxis = null;
    setState(() {});
  }

  @override
  void dispose() {
    _syncInset(false);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final size = MediaQuery.of(context).size;
    final dx = _dragAxis == Axis.horizontal ? _drag.dx : 0.0;
    final dy = _dragAxis == Axis.vertical ? _drag.dy : 0.0;
    final hFrac = dx / size.width;
    final vFrac = dy / size.height;

    final ps = PlaybackService.instance;
    return ValueListenableBuilder<PlayerState>(
      valueListenable: ps.playerState,
      builder: (context, state, _) {
        final isLoading = state.processingState == ProcessingState.loading;
        return ValueListenableBuilder(
          valueListenable: ps.queue,
          builder: (context, q, _) {
            final track = q.currentTrack;
            _syncInset(track != null || isLoading);
            if (track == null) {
              if (isLoading) {
                return Container(
                  height: 84,
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

            return TweenAnimationBuilder<Offset>(
              tween: Tween(begin: Offset.zero, end: Offset(dx, dy)),
              duration: (_drag == Offset.zero)
                  ? const Duration(milliseconds: 200)
                  : Duration.zero,
              curve: Curves.easeOut,
              builder: (context, offset, child) =>
                  Transform.translate(offset: offset, child: child),
              child: Opacity(
                opacity:
                    (1 - vFrac.abs()).clamp(0.4, 1.0) *
                    (1 - hFrac.abs()).clamp(0.6, 1.0),
                child: GestureDetector(
                  onTap: () => Navigator.push(
                    context,
                    MaterialPageRoute(builder: (_) => const PlayerPage()),
                  ),
                  onPanUpdate: _onPanUpdate,
                  onPanEnd: _onPanEnd,
                  onPanCancel: _onPanCancel,
                  child: Container(
                    height: 84,
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
                                const SeekBar(compact: true),
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
      width: AppTheme.spaceXXL * 3 * AppTheme.spaceScale(context),
      height: AppTheme.spaceXXL * 3 * AppTheme.spaceScale(context),
      child: Stack(
        children: [
          CoverImage(
            trackId: track.id,
            width: AppTheme.spaceXXL * 3 * AppTheme.spaceScale(context),
            height: AppTheme.spaceXXL * 3 * AppTheme.spaceScale(context),
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
