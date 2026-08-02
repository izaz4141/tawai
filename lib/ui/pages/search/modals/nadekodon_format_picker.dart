import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/helper.dart';

Future<String?> showNadekodonFormatPicker(
  BuildContext context, {
  required String infoJson,
}) async {
  final data = jsonDecode(infoJson) as Map<String, dynamic>;
  final items = data['items'] as List<dynamic>;
  final title = items.isNotEmpty ? items.first['name'] as String? ?? '' : '';
  final channel = items.isNotEmpty && items.first['thumbnail'] != null
      ? items.first['thumbnail'] as String?
      : null;

  final formats = <Map<String, dynamic>>[];
  for (final item in items) {
    final audios = item['audios'] as List<dynamic>? ?? [];
    for (final f in audios) {
      final m = f as Map<String, dynamic>;
      if (m['acodec'] != null && m['acodec'] != 'none') {
        formats.add(m);
      }
    }
  }
  formats.sort((a, b) => ((b['abr'] as num?) ?? 0).compareTo(
    (a['abr'] as num?) ?? 0,
  ));

  return showDialog<String>(
    context: context,
    builder: (ctx) {
      return SimpleDialog(
        title: Text(
          title,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        children: [
          if (channel != null)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: AppTheme.spaceMD),
              child: Text(
                'YouTube',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          const SizedBox(height: AppTheme.spaceXS),
          if (formats.isEmpty)
            const Padding(
              padding: EdgeInsets.all(AppTheme.spaceMD),
              child: Text('No audio formats available'),
            ),
          for (final f in formats)
            SimpleDialogOption(
              onPressed: () => Navigator.of(ctx).pop(f['format_id'] as String),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '${(f['abr'] as num?)?.toDouble().toStringAsFixed(0) ?? '?'} kbps — ${f['acodec'] ?? '?'}',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '${(f['note'] as String?)?.isNotEmpty == true ? '${f['note']} · ' : ''}'
                    '${f['filesize'] != null ? '${formatFileSize((f['filesize'] as num).toInt())} · ' : ''}'
                    '.${f['ext']}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
        ],
      );
    },
  );
}
