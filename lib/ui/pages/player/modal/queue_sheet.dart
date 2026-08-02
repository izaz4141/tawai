import 'package:flutter/material.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/src/bindings/bindings.dart';
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
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
            child: Row(
              children: [
                Text(
                  'Queue',
                  style: Theme.of(context)
                      .textTheme
                      .titleMedium
                      ?.copyWith(fontWeight: FontWeight.bold),
                ),
                const SizedBox(width: 8),
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
                          color: Theme.of(context)
                              .colorScheme
                              .primary
                              .withValues(alpha: 0.08),
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
                        icon: const Icon(Icons.close, size: 18),
                        onPressed: () =>
                            PlaybackService.instance.removeFromQueue(index),
                      ),
                      title: Text(
                        track.title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontWeight:
                              isCurrent ? FontWeight.w600 : FontWeight.normal,
                        ),
                      ),
                      subtitle: Text(
                        track.artistsString,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: () =>
                          PlaybackService.instance.playTrackAt(index),
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
