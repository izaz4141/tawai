import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/helper.dart';

class DownloadTile extends StatelessWidget {
  final DownloadRecord download;
  final VoidCallback? onCancel;

  const DownloadTile({super.key, required this.download, this.onCancel});

  @override
  Widget build(BuildContext context) {
    final totalSize = download.totalSize;
    final downloaded = download.downloaded;
    final progress = totalSize > 0 ? downloaded / totalSize : 0.0;
    final filename = download.filename.isNotEmpty
        ? download.filename
        : download.id;
    final state = download.state;
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Card(
      margin: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceMD,
        vertical: AppTheme.spaceXS,
      ),
      child: ListTile(
        title: Text(filename, maxLines: 1, overflow: TextOverflow.ellipsis),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                _buildSourceChip(download.source),
                const SizedBox(width: AppTheme.spaceSM),
                Text('State: $state'),
              ],
            ),
            if (totalSize > 0) ...[
              SizedBox(height: AppTheme.spaceXS),
              LinearProgressIndicator(value: progress),
              SizedBox(height: AppTheme.spaceXS / 2),
              Text(
                '${formatFileSize(downloaded)} / ${formatFileSize(totalSize)}',
                style: textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
            ],
          ],
        ),
        trailing: onCancel != null
            ? IconButton(
                icon: Icon(Icons.cancel_outlined, size: AppTheme.iconMD),
                onPressed: onCancel,
              )
            : null,
      ),
    );
  }

  Widget _buildSourceChip(String source) {
    final label = source == 'nadekodon'
        ? 'YouTube'
        : source == 'slskd'
        ? 'Soulseek'
        : source;
    final color = source == 'nadekodon'
        ? Colors.red.shade300
        : source == 'slskd'
        ? Colors.blue.shade300
        : Colors.grey;
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceSM - 2,
        vertical: AppTheme.spaceXS / 2,
      ),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(AppTheme.radiusSM / 2),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: AppTheme.textXS, color: color),
      ),
    );
  }
}
