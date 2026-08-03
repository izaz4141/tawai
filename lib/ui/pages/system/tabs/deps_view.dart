import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/utils/system_service.dart';

class SystemDeps extends StatelessWidget {
  const SystemDeps({super.key});

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final system = SystemService();

    return Column(
      children: [
        SectionHeader(
          title: 'Dependencies',
          leading: Icon(
            Icons.extension_rounded,
            color: colors.onPrimaryContainer,
            size: AppTheme.iconMD * AppTheme.iconScale(context),
          ),
        ),
        AnimatedBuilder(
          animation: Listenable.merge([
            system.ffmpegVersion,
            system.latestFfmpegVersion,
          ]),
          builder: (context, _) {
            final local = system.ffmpegVersion.value;
            final latest = system.latestFfmpegVersion.value?.version;
            final available = local != null;

            return ListTile(
              leading: Icon(
                Icons.movie_outlined,
                size: AppTheme.iconMD * AppTheme.iconScale(context),
              ),
              title: Text('ffmpeg', style: textTheme.bodyMedium),
              subtitle: Text(
                _formatWithLatest(local, latest, 'ffmpeg'),
                style: textTheme.bodySmall?.copyWith(
                  color: !available
                      ? Theme.of(context).colorScheme.error
                      : null,
                ),
              ),
              trailing: IconButton(
                icon: const Icon(Icons.open_in_new),
                iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
                tooltip: "Visit",
                onPressed: () =>
                    launchUrl(Uri.parse('https://github.com/FFmpeg/FFmpeg')),
              ),
            );
          },
        ),
        AnimatedBuilder(
          animation: Listenable.merge([
            system.slskdVersion,
            system.latestSlskdVersion,
          ]),
          builder: (context, _) {
            final local = system.slskdVersion.value;
            final latest = system.latestSlskdVersion.value?.version;
            final available = local != null;

            return ListTile(
              leading: Icon(
                Icons.cloud_outlined,
                size: AppTheme.iconMD * AppTheme.iconScale(context),
              ),
              title: Text('slskd', style: textTheme.bodyMedium),
              subtitle: Text(
                available
                    ? '$local (Latest: ${latest ?? "..."})'
                    : 'Not configured or unreachable',
                style: textTheme.bodySmall?.copyWith(
                  color: !available ? colors.error : null,
                ),
              ),
              trailing: IconButton(
                icon: const Icon(Icons.open_in_new),
                iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
                tooltip: "Visit",
                onPressed: () =>
                    launchUrl(Uri.parse('https://github.com/slskd/slskd')),
              ),
            );
          },
        ),
        AnimatedBuilder(
          animation: Listenable.merge([
            system.nadekodonVersion,
            system.latestNadekodonVersion,
          ]),
          builder: (context, _) {
            final local = system.nadekodonVersion.value;
            final latest = system.latestNadekodonVersion.value?.version;
            final available = local != null;

            return ListTile(
              leading: Icon(
                Icons.video_library_outlined,
                size: AppTheme.iconMD * AppTheme.iconScale(context),
              ),
              title: Text('nadekodon', style: textTheme.bodyMedium),
              subtitle: Text(
                available
                    ? '$local (Latest: ${latest ?? "..."})'
                    : 'Not configured or unreachable',
                style: textTheme.bodySmall?.copyWith(
                  color: !available ? colors.error : null,
                ),
              ),
              trailing: IconButton(
                icon: const Icon(Icons.open_in_new),
                iconSize: AppTheme.iconMD * AppTheme.iconScale(context),
                tooltip: "Visit",
                onPressed: () => launchUrl(
                  Uri.parse('https://github.com/izaz4141/nadekodon-rs'),
                ),
              ),
            );
          },
        ),
      ],
    );
  }
}

String _formatWithLatest(String? local, String? latest, String name) {
  if (local == null) return 'Checking... ($name will not work)';
  return '$local (Latest: ${latest ?? "..."})';
}
