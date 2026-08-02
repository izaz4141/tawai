import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class TrackListView extends StatelessWidget {
  final List<TrackInfo> tracks;
  final bool loading;
  final String? selectedTrackId;
  final ValueChanged<TrackInfo> onSelect;
  final VoidCallback onRefresh;

  const TrackListView({
    super.key,
    required this.tracks,
    required this.loading,
    this.selectedTrackId,
    required this.onSelect,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    if (loading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (tracks.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.check_circle_outline, size: 48, color: colors.outline),
            const SizedBox(height: AppTheme.spaceMD),
            Text('No tracks', style: Theme.of(context).textTheme.bodyLarge),
            const SizedBox(height: AppTheme.spaceSM),
            FilledButton.tonal(
              onPressed: onRefresh,
              child: const Text('Refresh'),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: () async => onRefresh(),
      child: ListView.builder(
        itemCount: tracks.length,
        itemBuilder: (context, index) {
          final t = tracks[index];
          final selected = selectedTrackId == t.id;
          return ListTile(
            dense: true,
            selected: selected,
            selectedTileColor: colors.primaryContainer.withAlpha(100),
            leading: Icon(
              Icons.disc_full,
              size: AppTheme.iconSM,
              color: t.mbidRecording != null ? Colors.green : colors.error,
            ),
            title: Text(t.title, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${t.artistsString} — ${t.albumTitle}',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            trailing: selected
                ? Icon(Icons.chevron_right, color: colors.primary, size: AppTheme.iconMD)
                : null,
            onTap: () => onSelect(t),
          );
        },
      ),
    );
  }
}
