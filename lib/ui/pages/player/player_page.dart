import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';

import 'package:tawai/services/playback_service.dart';
import 'package:tawai/models/recommendation_source.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/widgets/components/playback_button.dart';
import 'package:tawai/ui/widgets/components/volume_button.dart';
import 'package:tawai/ui/widgets/components/seek_bar.dart';
import 'package:tawai/ui/widgets/components/track_action_sheet.dart';
import 'package:tawai/ui/widgets/components/download_track_sheet.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/player/modal/queue_sheet.dart';
import 'package:tawai/ui/pages/player/subpages/lyrics/lyrics_page.dart';
import 'package:tawai/ui/pages/player/widgets/lyrics_view.dart';
import 'package:tawai/ui/pages/player/widgets/queue_view.dart';
import 'package:tawai/utils/platform_service.dart';

const double kBottomPanelFraction = 0.4;

enum _BottomPanel { queue, lyrics }

class PlayerPage extends StatefulWidget {
  const PlayerPage({super.key});

  @override
  State<PlayerPage> createState() => _PlayerPageState();
}

class _PlayerPageState extends State<PlayerPage> {
  _BottomPanel? _activePanel;
  final ScrollController _bottomScroll = ScrollController();

  @override
  void dispose() {
    _bottomScroll.dispose();
    super.dispose();
  }

  void _togglePanel(_BottomPanel panel) {
    if (_activePanel == panel) {
      switch (panel) {
        case _BottomPanel.queue:
          showModalBottomSheet(
            context: context,
            isScrollControlled: true,
            builder: (_) => const QueueSheet(),
          );
        case _BottomPanel.lyrics:
          Navigator.push(
            context,
            MaterialPageRoute(builder: (_) => const LyricsPage()),
          );
      }
      return;
    }
    setState(() => _activePanel = panel);
  }

  void _closePanel() {
    setState(() => _activePanel = null);
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;

    return Scaffold(
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
                  return Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SizedBox(
                          width:
                              AppTheme.spaceXXL * AppTheme.spaceScale(context),
                          height:
                              AppTheme.spaceXXL * AppTheme.spaceScale(context),
                          child: const CircularProgressIndicator(
                            strokeWidth: 3,
                          ),
                        ),
                        SizedBox(
                          height:
                              AppTheme.spaceSM *
                              2 *
                              AppTheme.spaceScale(context),
                        ),
                        const Text('Loading track...'),
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
                    fit: FlexFit.loose,
                    flex: 3,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: AppTheme.spaceXL * 2,
                        vertical: AppTheme.spaceXL,
                      ),
                      child: Center(
                        heightFactor: 1,
                        widthFactor: 1,
                        child: AspectRatio(
                          aspectRatio: 1,
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(
                              AppTheme.radiusSM * 2,
                            ),
                            child: CoverImage(
                              trackId: track.id,
                              fit: BoxFit.contain,
                              iconSize:
                                  AppTheme.iconXXL *
                                  AppTheme.iconScale(context),
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
                    child: Row(
                      children: [
                        IconButton(
                          icon: Icon(
                            Icons.menu,
                            size: AppTheme.iconMD * AppTheme.iconScale(context),
                          ),
                          onPressed: () => showTrackActionSheet(context, track),
                        ),
                        Expanded(
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
                        VolumeButton(
                          iconSize:
                              AppTheme.iconMD * AppTheme.iconScale(context),
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
                    child: SeekBar(),
                  ),
                  const _PlayerControlsRow(),
                  SizedBox(
                    height: AppTheme.iconXL,
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        _PanelToggleButton(
                          icon: Icons.queue_music,
                          label: 'Queue',
                          active: _activePanel == _BottomPanel.queue,
                          onPressed: () => _togglePanel(_BottomPanel.queue),
                        ),
                        const SizedBox(width: AppTheme.spaceXXL),
                        _PanelToggleButton(
                          icon: Icons.lyrics_outlined,
                          label: 'Lyrics',
                          active: _activePanel == _BottomPanel.lyrics,
                          onPressed: () => _togglePanel(_BottomPanel.lyrics),
                        ),
                        if (RecommendationSource.isDownloadable(
                          track.sourceType,
                        )) ...[
                          const SizedBox(width: AppTheme.spaceXXL),
                          TextButton.icon(
                            icon: const Icon(Icons.download),
                            label: const Text('Download'),
                            onPressed: () =>
                                showTrackDownloadSheet(context, track),
                          ),
                        ],
                      ],
                    ),
                  ),
                  if (_activePanel == null)
                    SizedBox(
                      height:
                          AppTheme.spaceXXL * 2 * AppTheme.spaceScale(context),
                    ),
                  AnimatedSize(
                    duration: const Duration(milliseconds: 250),
                    curve: Curves.easeOut,
                    alignment: Alignment.topCenter,
                    child: _activePanel == null
                        ? const SizedBox(width: double.infinity)
                        : SizedBox(
                            height:
                                MediaQuery.sizeOf(context).height *
                                kBottomPanelFraction,
                            child: _buildPanel(context, q),
                          ),
                  ),
                ],
              );
            },
          );
        },
      ),
    );
  }

  Widget _buildPanel(BuildContext context, QueueState q) {
    final colors = Theme.of(context).colorScheme;
    final spaceScale = AppTheme.spaceScale(context);
    final isQueue = _activePanel == _BottomPanel.queue;

    return Column(
      children: [
        Padding(
          padding: EdgeInsets.fromLTRB(
            AppTheme.spaceSM * 2 * spaceScale,
            AppTheme.spaceSM * spaceScale,
            AppTheme.spaceSM * spaceScale,
            AppTheme.spaceXS * spaceScale,
          ),
          child: Row(
            children: [
              Text(
                isQueue ? 'Queue' : 'Lyrics',
                style: Theme.of(
                  context,
                ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.bold),
              ),
              if (isQueue) ...[
                SizedBox(width: AppTheme.spaceSM * spaceScale),
                Text(
                  '${q.items.length} track${q.items.length == 1 ? '' : 's'}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
              const Spacer(),
              IconButton(
                icon: Icon(
                  Icons.close,
                  size: AppTheme.iconSM * AppTheme.iconScale(context),
                ),
                color: colors.onSurfaceVariant,
                visualDensity: VisualDensity.compact,
                onPressed: _closePanel,
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: isQueue
              ? QueueView(
                  items: q.items,
                  currentIndex: q.currentIndex,
                  scrollController: _bottomScroll,
                  onRemove: PlaybackService.instance.removeFromQueue,
                  onReorder: PlaybackService.instance.moveQueueItem,
                  onTap: PlaybackService.instance.playTrackAt,
                )
              : const LyricsView(),
        ),
      ],
    );
  }
}

class _PanelToggleButton extends StatelessWidget {
  const _PanelToggleButton({
    required this.icon,
    required this.label,
    required this.active,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final bool active;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return TextButton.icon(
      icon: Icon(icon),
      label: Text(label),
      style: TextButton.styleFrom(
        foregroundColor: active ? colors.primary : null,
        backgroundColor: active ? colors.primary.withValues(alpha: 0.1) : null,
      ),
      onPressed: onPressed,
    );
  }
}

class _PlayerControlsRow extends StatelessWidget {
  const _PlayerControlsRow();

  @override
  Widget build(BuildContext context) {
    final ps = PlaybackService.instance;
    final iconScale = AppTheme.iconScale(context);
    return SizedBox(
      height: AppTheme.spaceSM * 10 * AppTheme.spaceScale(context),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          ValueListenableBuilder(
            valueListenable: ps.shuffleModeEnabled,
            builder: (context, shuffle, _) {
              return IconButton(
                icon: Icon(
                  Icons.shuffle,
                  size: AppTheme.iconMD * iconScale,
                  color: shuffle ? Theme.of(context).colorScheme.primary : null,
                ),
                onPressed: () => ps.toggleShuffle(),
              );
            },
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          IconButton(
            icon: Icon(
              Icons.skip_previous_rounded,
              size: (AppTheme.iconLG + AppTheme.spaceXS) * iconScale,
            ),
            onPressed: () => ps.playPrevious(),
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          PlaybackButton(
            iconSize: AppTheme.iconXXL * iconScale,
            color: Theme.of(context).colorScheme.primary,
            playIcon: Icons.play_circle_fill_rounded,
            pauseIcon: Icons.pause_circle_filled_rounded,
          ),
          const SizedBox(width: AppTheme.spaceSM * 2),
          IconButton(
            icon: Icon(
              Icons.skip_next_rounded,
              size: (AppTheme.iconLG + AppTheme.spaceXS) * iconScale,
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
                icon: Icon(
                  icon,
                  size: AppTheme.iconMD * iconScale,
                  color: color,
                ),
                onPressed: ps.cycleLoopMode,
              );
            },
          ),
        ],
      ),
    );
  }
}
