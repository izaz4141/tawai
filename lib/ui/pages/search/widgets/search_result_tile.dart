import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/search/search_types.dart';
import 'package:tawai/utils/helper.dart';
import 'package:tawai/ui/widgets/components/cached_network_image.dart';

class SearchResultTile extends StatelessWidget {
  final SearchResultItem entry;
  final VoidCallback onDownload;
  final Set<String> loadingUrls;

  const SearchResultTile({
    super.key,
    required this.entry,
    required this.onDownload,
    this.loadingUrls = const {},
  });

  @override
  Widget build(BuildContext context) {
    if (entry.sourceType == 'slskd') {
      return _buildSlskdTile(context);
    }
    return _buildNadekodonTile(context);
  }

  Widget _buildSlskdTile(BuildContext context) {
    return ListTile(
      leading: const Icon(Icons.audio_file),
      title: Text(
        entry.filename.split('/').last,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        '${entry.username} · ${formatFileSize(entry.size)}'
        ' · ${entry.bitrate} kbps'
        ' · ${entry.extension}',
      ),
      trailing: IconButton(
        icon: const Icon(Icons.download),
        onPressed: onDownload,
      ),
    );
  }

  Widget _buildNadekodonTile(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Card(
      margin: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceMD,
        vertical: AppTheme.spaceXS,
      ),
      child: ListTile(
        leading: entry.thumbnail != null
            ? ClipRRect(
                borderRadius: BorderRadius.circular(AppTheme.radiusSM / 2),
                child: CachedNetworkImage(
                  url: entry.thumbnail!,
                  width: AppTheme.iconXL,
                  height: AppTheme.iconXL,
                  fit: BoxFit.cover,
                  placeholder: const Icon(Icons.video_file),
                ),
              )
            : const Icon(Icons.video_file),
        title: Text(
          entry.title ?? entry.filename,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              entry.channel ?? 'Unknown channel',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            if (entry.duration != null)
              Text(
                formatDuration(entry.duration),
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: colors.outline),
              ),
          ],
        ),
        trailing: loadingUrls.contains(entry.filename)
            ? SizedBox(
                width: AppTheme.iconXL * AppTheme.iconScale(context),
                height: AppTheme.iconXL * AppTheme.iconScale(context),
                child: const Center(
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            : IconButton(
                icon: const Icon(Icons.download),
                onPressed: onDownload,
              ),
      ),
    );
  }
}
