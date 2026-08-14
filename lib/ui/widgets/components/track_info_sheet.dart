import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/utils/helper.dart';

void showTrackInfoSheet(BuildContext context, TrackInfo track) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _TrackInfoSheet(track: track),
  );
}

class _TrackInfoSheet extends StatelessWidget {
  const _TrackInfoSheet({required this.track});

  final TrackInfo track;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Sheet(
      header: Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceXL,
          AppTheme.spaceMD,
          AppTheme.spaceXL,
          AppTheme.spaceXS,
        ),
        child: Row(
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(AppTheme.radiusSM),
              child: SizedBox(
                width: AppTheme.iconXL,
                height: AppTheme.iconXL,
                child: CoverImage(trackId: track.id, iconSize: AppTheme.iconMD),
              ),
            ),
            const SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(track.title, style: theme.textTheme.titleMedium),
                  if (track.artistsString.isNotEmpty ||
                      track.albumTitle.isNotEmpty)
                    Text(
                      '${track.artistsString}${track.albumTitle.isNotEmpty ? ' · ${track.albumTitle}' : ''}',
                      style: theme.textTheme.bodySmall,
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
      body: Padding(
        padding: const EdgeInsets.fromLTRB(
          AppTheme.spaceXL,
          AppTheme.spaceSM,
          AppTheme.spaceXL,
          AppTheme.spaceXL,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _SectionHeader(title: 'Track'),
            _InfoRow(label: 'Title', value: track.title),
            _InfoRow(label: 'Artist', value: track.artistsString),
            _InfoRow(label: 'Album', value: track.albumTitle),
            _InfoRow(label: 'Track #', value: track.trackNum?.toString()),
            _InfoRow(label: 'Disc #', value: track.discNum?.toString()),
            _InfoRow(
              label: 'Duration',
              value: formatDuration(track.durationSecs),
            ),
            _InfoRow(
              label: 'Genres',
              value: track.genres.isEmpty ? null : track.genres.join(', '),
            ),
            _InfoRow(label: 'Release Date', value: track.releaseDate),
            const Divider(height: AppTheme.spaceLG),
            _SectionHeader(title: 'File'),
            _InfoRow(label: 'Source', value: track.source),
            _InfoRow(label: 'Source Type', value: track.sourceType),
            _InfoRow(
              label: 'Bitrate',
              value: track.bitrate != null ? '${track.bitrate} kbps' : null,
            ),
            _InfoRow(
              label: 'File Size',
              value: track.fileSize != null
                  ? formatFileSize(track.fileSize!)
                  : null,
            ),
            _InfoRow(label: 'File Path', value: track.filePath),
            const Divider(height: AppTheme.spaceLG),
            _SectionHeader(title: 'Details'),
            _InfoRow(label: 'Track ID', value: track.id),
            _InfoRow(label: 'Album ID', value: track.albumId),
            _InfoRow(label: 'Recording MBID', value: track.mbidRecording),
            _InfoRow(label: 'Artist MBID', value: track.artistMbid),
            _InfoRow(label: 'Album MBID', value: track.albumMbid),
            _InfoRow(
              label: 'ReplayGain',
              value: track.trackGain != null ? '${track.trackGain} dB' : null,
            ),
            _InfoRow(label: 'ReplayPeak', value: track.trackPeak?.toString()),
          ],
        ),
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(
        bottom: AppTheme.spaceXS,
        top: AppTheme.spaceXS,
      ),
      child: Text(
        title.toUpperCase(),
        style: theme.textTheme.titleSmall?.copyWith(
          color: theme.colorScheme.primary,
          letterSpacing: 1.2,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String? value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: AppTheme.spaceXS),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: AppTheme.spaceXL * 4.2,
            child: Text(
              label,
              style: theme.textTheme.labelMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: SelectableText(
              value?.isNotEmpty == true ? value! : '—',
              style: theme.textTheme.bodyMedium,
            ),
          ),
        ],
      ),
    );
  }
}
