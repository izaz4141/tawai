import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/models/stream_info.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';

/// Compact, tappable line showing how the current track is being played.
///
/// Only values detected by the backend probe are shown. While the probe runs it
/// reads e.g. `Transcode · Analyzing…`; when probing fails it degrades to the
/// mode alone and the detail sheet explains that details are unavailable.
class StreamInfoBar extends StatelessWidget {
  const StreamInfoBar({super.key, this.info});

  /// Test/override hook; defaults to the live playback service notifier.
  final ValueListenable<StreamInfo?>? info;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final listenable = info ?? PlaybackService.instance.streamInfo;

    return ValueListenableBuilder<StreamInfo?>(
      valueListenable: listenable,
      builder: (context, info, _) {
        if (info == null) return const SizedBox.shrink();

        return Padding(
          padding: const EdgeInsets.only(top: AppTheme.spaceXS),
          child: Center(
            child: InkWell(
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              onTap: () => showStreamInfoSheet(context, listenable),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: AppTheme.spaceSM,
                  vertical: AppTheme.spaceXS,
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (info.loading)
                      SizedBox(
                        width: AppTheme.iconSM * 0.7,
                        height: AppTheme.iconSM * 0.7,
                        child: CircularProgressIndicator(
                          strokeWidth: 1.5,
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    else
                      Icon(
                        Icons.graphic_eq,
                        size: AppTheme.iconSM,
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    const SizedBox(width: AppTheme.spaceXS),
                    Flexible(
                      child: Text(
                        info.summary,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    Icon(
                      Icons.chevron_right,
                      size: AppTheme.iconSM,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

void showStreamInfoSheet(
  BuildContext context,
  ValueListenable<StreamInfo?> info,
) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _StreamInfoSheet(info: info),
  );
}

class _StreamInfoSheet extends StatelessWidget {
  const _StreamInfoSheet({required this.info});

  final ValueListenable<StreamInfo?> info;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Sheet(
      header: Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceXL,
          AppTheme.spaceSM,
          AppTheme.spaceXL,
          AppTheme.spaceSM,
        ),
        child: Row(
          children: [
            Expanded(
              child: Text(
                'Stream details',
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.close),
              visualDensity: VisualDensity.compact,
              onPressed: () => Navigator.of(context).pop(),
            ),
          ],
        ),
      ),
      body: ValueListenableBuilder<StreamInfo?>(
        valueListenable: info,
        builder: (context, info, _) {
          if (info == null) {
            return Padding(
              padding: const EdgeInsets.fromLTRB(
                AppTheme.spaceXL,
                AppTheme.spaceSM,
                AppTheme.spaceXL,
                AppTheme.spaceSM,
              ),
              child: Text(
                'No stream information available.',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            );
          }

          return ListView(
            shrinkWrap: true,
            padding: const EdgeInsets.symmetric(vertical: AppTheme.spaceSM),
            children: [
              for (final row in info.detailRows)
                ListTile(
                  dense: true,
                  title: Text(row.label),
                  trailing: ConstrainedBox(
                    constraints: BoxConstraints(
                      maxWidth: MediaQuery.sizeOf(context).width * 0.6,
                    ),
                    child: Text(
                      row.value,
                      textAlign: TextAlign.end,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ),
              if (!info.loading && !info.hasDetails)
                Padding(
                  padding: const EdgeInsets.fromLTRB(
                    AppTheme.spaceXL,
                    AppTheme.spaceSM,
                    AppTheme.spaceXL,
                    AppTheme.spaceSM,
                  ),
                  child: Text(
                    'The stream could not be inspected. Playback mode is known, '
                    'but codec, bitrate, and sample rate are unavailable.',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
            ],
          );
        },
      ),
    );
  }
}
