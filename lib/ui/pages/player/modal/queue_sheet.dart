import 'package:flutter/material.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';

class QueueSheet extends StatelessWidget {
  const QueueSheet({super.key});

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder(
      valueListenable: PlaybackService.instance.queue,
      builder: (context, q, _) {
        final items = q.items;
        final currentIdx = q.currentIndex;
        return Sheet(
          draggable: true,
          initialChildSize: 0.6,
          minChildSize: 0.3,
          maxChildSize: 0.9,
          header: Padding(
            padding: EdgeInsets.fromLTRB(
              AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              AppTheme.spaceMD * AppTheme.spaceScale(context),
              AppTheme.spaceSM * 2 * AppTheme.spaceScale(context),
              AppTheme.spaceXS * AppTheme.spaceScale(context),
            ),
            child: Row(
              children: [
                Text(
                  'Queue',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                SizedBox(
                  width: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                Text(
                  '${items.length} track${items.length == 1 ? '' : 's'}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const Spacer(),
                if (items.isNotEmpty)
                  TextButton(
                    onPressed: () => PlaybackService.instance.clearQueue(),
                    child: const Text('Clear'),
                  ),
              ],
            ),
          ),
          bodyBuilder: (scrollCtrl) {
            if (items.isEmpty) {
              return const Center(child: Text('Queue is empty'));
            }
            return ReorderableListView.builder(
              scrollController: scrollCtrl,
              itemCount: items.length,
              onReorderItem: (from, to) =>
                  PlaybackService.instance.moveQueueItem(from, to),
              itemBuilder: (context, index) {
                final track = items[index].track;
                final isCurrent = index == currentIdx;
                return Container(
                  key: ValueKey('${track.id}-$index'),
                  decoration: isCurrent
                      ? BoxDecoration(
                          color: Theme.of(
                            context,
                          ).colorScheme.primary.withValues(alpha: 0.08),
                          border: Border(
                            left: BorderSide(
                              color: Theme.of(context).colorScheme.primary,
                              width: 3,
                            ),
                          ),
                        )
                      : null,
                  child: Material(
                    type: MaterialType.transparency,
                    child: ListTile(
                      leading: IconButton(
                        icon: Icon(
                          Icons.close,
                          size: AppTheme.iconSM * AppTheme.iconScale(context),
                        ),
                        onPressed: () =>
                            PlaybackService.instance.removeFromQueue(index),
                      ),
                      title: Text(
                        track.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontWeight: isCurrent
                              ? FontWeight.w600
                              : FontWeight.normal,
                        ),
                      ),
                      subtitle: Text(
                        track.artistsString,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: () => PlaybackService.instance.playTrackAt(index),
                    ),
                  ),
                );
              },
            );
          },
        );
      },
    );
  }
}
