import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/utils/helper.dart';

const double kQueueTileHeight = AppTheme.spaceSM * 10;

class QueueView extends StatefulWidget {
  const QueueView({
    super.key,
    required this.items,
    required this.currentIndex,
    required this.scrollController,
    required this.onRemove,
    required this.onReorder,
    required this.onTap,
  });

  final List<QueueItem> items;
  final int currentIndex;
  final ScrollController scrollController;
  final void Function(int index) onRemove;
  final void Function(int from, int to) onReorder;
  final void Function(int index) onTap;

  @override
  State<QueueView> createState() => _QueueViewState();
}

class _QueueViewState extends State<QueueView> {
  final GlobalKey _currentKey = GlobalKey();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _revealCurrent());
  }

  /// On mount, scroll the queue so the current track sits at the very top.
  /// First jumps to an index-based estimate so the (possibly lazy) current
  /// tile gets built, then snaps it exactly to the top edge.
  void _revealCurrent() {
    final idx = widget.currentIndex;
    final ctrl = widget.scrollController;
    if (idx <= 0 || !ctrl.hasClients) return;

    final estimate = (idx * kQueueTileHeight).clamp(
      0.0,
      ctrl.position.maxScrollExtent,
    );
    if ((ctrl.offset - estimate).abs() > 0.5) {
      ctrl.jumpTo(estimate);
    }

    WidgetsBinding.instance.addPostFrameCallback((_) {
      final ctx = _currentKey.currentContext;
      if (ctx != null) {
        Scrollable.ensureVisible(
          ctx,
          alignment: 0.0,
          duration: const Duration(milliseconds: 250),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    if (widget.items.isEmpty) {
      final colors = Theme.of(context).colorScheme;
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.queue_music,
              size: AppTheme.iconLG * AppTheme.iconScale(context),
              color: colors.onSurfaceVariant,
            ),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            Text(
              'Queue is empty',
              style: Theme.of(
                context,
              ).textTheme.bodyLarge?.copyWith(color: colors.onSurfaceVariant),
            ),
          ],
        ),
      );
    }

    return ReorderableListView.builder(
      scrollController: widget.scrollController,
      itemCount: widget.items.length,
      itemExtent: kQueueTileHeight,
      buildDefaultDragHandles: false,
      onReorderItem: widget.onReorder,
      itemBuilder: (context, index) {
        final track = widget.items[index].track;
        final isCurrent = index == widget.currentIndex;
        return _buildTile(context, track, index, isCurrent);
      },
    );
  }

  Widget _buildTile(
    BuildContext context,
    TrackInfo track,
    int index,
    bool isCurrent,
  ) {
    return KeyedSubtree(
      key: ValueKey('${track.id}-$index'),
      child: ReorderableDelayedDragStartListener(
        index: index,
        child: Row(
          children: [
            Expanded(
              child: isCurrent
                  ? _CurrentTrackTile(
                      key: _currentKey,
                      track: track,
                      onTap: () => PlaybackService.instance.togglePlayPause(),
                      onRemove: () => widget.onRemove(index),
                    )
                  : _QueueTrackTile(
                      track: track,
                      onTap: () => widget.onTap(index),
                      onRemove: () => widget.onRemove(index),
                    ),
            ),
            ReorderableDragStartListener(
              index: index,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: AppTheme.spaceXS,
                  vertical: AppTheme.spaceSM,
                ),
                child: MouseRegion(
                  cursor: SystemMouseCursors.grab,
                  child: Icon(
                    Icons.drag_handle,
                    size: AppTheme.iconSM * AppTheme.iconScale(context),
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _QueueTrackTile extends StatelessWidget {
  const _QueueTrackTile({
    super.key,
    required this.track,
    required this.onTap,
    required this.onRemove,
  });

  final TrackInfo track;
  final VoidCallback onTap;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final spaceScale = AppTheme.spaceScale(context);
    final coverSize = AppTheme.spaceSM * 5 * spaceScale;

    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          height: kQueueTileHeight,
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: AppTheme.spaceSM * spaceScale,
            ),
            child: Row(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(
                    AppTheme.radiusSM * AppTheme.radiusScale(context),
                  ),
                  child: SizedBox(
                    width: coverSize,
                    height: coverSize,
                    child: CoverImage(
                      trackId: track.id,
                      iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
                    ),
                  ),
                ),
                SizedBox(width: AppTheme.spaceSM * 1.5 * spaceScale),
                Expanded(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        track.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '${track.artistsString} · ${track.albumTitle}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                SizedBox(width: AppTheme.spaceSM * spaceScale),
                Text(
                  formatDuration(track.durationSecs),
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
                IconButton(
                  icon: Icon(
                    Icons.close,
                    size: AppTheme.iconSM * AppTheme.iconScale(context),
                  ),
                  color: colors.onSurfaceVariant,
                  visualDensity: VisualDensity.compact,
                  onPressed: onRemove,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CurrentTrackTile extends StatelessWidget {
  const _CurrentTrackTile({
    super.key,
    required this.track,
    required this.onTap,
    required this.onRemove,
  });

  final TrackInfo track;
  final VoidCallback onTap;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final spaceScale = AppTheme.spaceScale(context);
    final radiusScale = AppTheme.radiusScale(context);
    final iconScale = AppTheme.iconScale(context);
    final coverSize = AppTheme.spaceSM * 5 * spaceScale;

    return Material(
      type: MaterialType.transparency,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(
          AppTheme.radiusSM * 2 * radiusScale,
        ),
        child: SizedBox(
          height: kQueueTileHeight,
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: AppTheme.spaceSM * spaceScale,
              vertical: 2,
            ),
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 250),
              curve: Curves.easeOut,
              decoration: BoxDecoration(
                color: colors.primaryContainer,
                borderRadius: BorderRadius.circular(
                  AppTheme.radiusSM * 2 * radiusScale,
                ),
                border: Border.all(color: colors.primary, width: 1.5),
                boxShadow: [
                  BoxShadow(
                    color: colors.primary.withValues(alpha: 0.28),
                    blurRadius: 12,
                    offset: const Offset(0, 2),
                  ),
                ],
              ),
              child: Row(
                children: [
                  Container(
                    width: AppTheme.spaceXS,
                    height: AppTheme.spaceLG * 2,
                    margin: EdgeInsets.only(
                      left: AppTheme.spaceSM * spaceScale,
                    ),
                    decoration: BoxDecoration(
                      color: colors.primary,
                      borderRadius: BorderRadius.circular(AppTheme.radiusSM),
                    ),
                  ),
                  SizedBox(width: AppTheme.spaceSM * spaceScale),
                  _HeroCover(track: track, size: coverSize),
                  SizedBox(width: AppTheme.spaceSM * 1.5 * spaceScale),
                  Expanded(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Container(
                          padding: EdgeInsets.symmetric(
                            horizontal: AppTheme.spaceSM * spaceScale,
                            vertical: 1,
                          ),
                          decoration: BoxDecoration(
                            color: colors.primary,
                            borderRadius: BorderRadius.circular(
                              AppTheme.radiusSM * radiusScale,
                            ),
                          ),
                          child: Text(
                            'NOW PLAYING',
                            style: Theme.of(context).textTheme.labelSmall
                                ?.copyWith(
                                  fontSize:
                                      AppTheme.textXS *
                                      AppTheme.textScale(context),
                                  fontWeight: FontWeight.w700,
                                  letterSpacing: 0.8,
                                  color: colors.onPrimary,
                                ),
                          ),
                        ),
                        SizedBox(height: AppTheme.spaceXS * spaceScale),
                        Flexible(
                          fit: FlexFit.loose,
                          child: Text(
                            track.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.titleSmall
                                ?.copyWith(
                                  color: colors.primary,
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                        ),
                        const SizedBox(height: 1),
                        Flexible(
                          fit: FlexFit.loose,
                          child: Text(
                            '${track.artistsString} · ${track.albumTitle}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: colors.onPrimaryContainer),
                          ),
                        ),
                        SizedBox(height: AppTheme.spaceXS * spaceScale),
                        const _CurrentTrackProgress(),
                      ],
                    ),
                  ),
                  SizedBox(width: AppTheme.spaceSM * spaceScale),
                  ValueListenableBuilder<PlayerState>(
                    valueListenable: PlaybackService.instance.playerState,
                    builder: (context, state, _) {
                      return IconButton.filled(
                        color: colors.onPrimary,
                        iconSize: AppTheme.iconMD * iconScale,
                        onPressed: () =>
                            PlaybackService.instance.togglePlayPause(),
                        icon: Icon(
                          state.playing
                              ? Icons.pause_rounded
                              : Icons.play_arrow_rounded,
                        ),
                      );
                    },
                  ),
                  IconButton(
                    icon: Icon(Icons.close, size: AppTheme.iconSM * iconScale),
                    color: colors.onPrimaryContainer,
                    visualDensity: VisualDensity.compact,
                    onPressed: onRemove,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _HeroCover extends StatelessWidget {
  const _HeroCover({required this.track, required this.size});

  final TrackInfo track;
  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final radius = AppTheme.radiusSM * AppTheme.radiusScale(context);

    return SizedBox(
      width: size,
      height: size,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(radius),
              border: Border.all(color: colors.primary, width: 2),
              boxShadow: [
                BoxShadow(
                  color: colors.primary.withValues(alpha: 0.4),
                  blurRadius: 8,
                ),
              ],
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(radius - 2),
              child: SizedBox(
                width: size,
                height: size,
                child: CoverImage(
                  trackId: track.id,
                  iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
                ),
              ),
            ),
          ),
          Positioned(
            right: -6,
            bottom: -6,
            child: ValueListenableBuilder<PlayerState>(
              valueListenable: PlaybackService.instance.playerState,
              builder: (context, state, _) =>
                  _EqualizerBars(playing: state.playing),
            ),
          ),
        ],
      ),
    );
  }
}

class _CurrentTrackProgress extends StatelessWidget {
  const _CurrentTrackProgress();

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return ValueListenableBuilder<Duration>(
      valueListenable: PlaybackService.instance.position,
      builder: (context, pos, _) {
        return ValueListenableBuilder<Duration>(
          valueListenable: PlaybackService.instance.duration,
          builder: (context, dur, _) {
            final value = dur.inMilliseconds > 0
                ? (pos.inMilliseconds / dur.inMilliseconds).clamp(0.0, 1.0)
                : 0.0;
            return ClipRRect(
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              child: LinearProgressIndicator(
                value: value,
                minHeight: 3,
                backgroundColor: colors.primary.withValues(alpha: 0.15),
                color: colors.primary,
              ),
            );
          },
        );
      },
    );
  }
}

class _EqualizerBars extends StatefulWidget {
  const _EqualizerBars({required this.playing});

  final bool playing;

  @override
  State<_EqualizerBars> createState() => _EqualizerBarsState();
}

class _EqualizerBarsState extends State<_EqualizerBars>
    with SingleTickerProviderStateMixin {
  static const List<double> _phases = [0, 1.2, 2.4];

  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 800),
  );

  @override
  void initState() {
    super.initState();
    _sync();
  }

  @override
  void didUpdateWidget(_EqualizerBars oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.playing != widget.playing) _sync();
  }

  void _sync() {
    if (widget.playing) {
      _controller.repeat(reverse: true);
    } else {
      _controller.stop();
      _controller.value = 0.5;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: colors.primary,
        shape: BoxShape.circle,
        border: Border.all(color: colors.surface, width: 1.5),
      ),
      child: SizedBox(
        width: 14,
        height: 14,
        child: AnimatedBuilder(
          animation: _controller,
          builder: (context, _) {
            final t = _controller.value;
            return Row(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                for (var i = 0; i < 3; i++)
                  Container(
                    width: 2,
                    height: widget.playing
                        ? 3 +
                              8 *
                                  (0.5 +
                                      0.5 *
                                          math.sin(
                                            (t * 2 * math.pi) + _phases[i],
                                          ))
                        : 6,
                    margin: const EdgeInsets.symmetric(horizontal: 1),
                    decoration: BoxDecoration(
                      color: colors.onPrimary,
                      borderRadius: BorderRadius.circular(1),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}
