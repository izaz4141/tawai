import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_button.dart';
import 'package:tawai/ui/widgets/components/list_dropdown.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/system_service.dart';

class SettingsDownloadsTab extends StatefulWidget {
  const SettingsDownloadsTab({super.key});

  @override
  State<SettingsDownloadsTab> createState() => _SettingsDownloadsTabState();
}

class _SettingsDownloadsTabState extends State<SettingsDownloadsTab> {
  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder(
      valueListenable: SettingsManager.currentUser,
      builder: (context, user, _) {
        final isAdmin = user?.role == 'admin';
        if (!isAdmin) {
          return _AdminOnly();
        }

        return ListView(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
            vertical: AppTheme.spaceLG,
          ),
          children: [
            const SectionHeader(
              title: 'Destination',
              leading: Icon(Icons.folder_outlined),
            ),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            ValueListenableBuilder<String>(
              valueListenable: SettingsManager.downloadFolder,
              builder: (context, path, _) {
                Future<void> pick() async {
                  final p = await IOServiceFactory.create().getDirectoryPath(
                    context,
                    initialPath: path.isNotEmpty ? path : null,
                  );
                  if (p != null && context.mounted) {
                    SettingsManager.downloadFolder.value = p;
                  }
                }

                return ListButton(
                  title: 'Download Folder',
                  subtitle: path.isEmpty ? 'Not set' : path,
                  enabled: true,
                  onPressed: pick,
                  type: ListButtonType.text,
                  label: 'Browse',
                );
              },
            ),
            SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
            const SectionHeader(
              title: 'Defaults',
              leading: Icon(Icons.download),
            ),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            ListenableBuilder(
              listenable: Listenable.merge([
                SystemService().slskdVersion,
                SystemService().nadekodonVersion,
              ]),
              builder: (context, _) {
                final sys = SystemService();
                final unconfigured = <String>[
                  if (sys.slskdVersion.value == null) 'Soulseek',
                  if (sys.nadekodonVersion.value == null) 'YouTube',
                ];
                return ListDropdown(
                  title: 'Download Source',
                  subtitle: unconfigured.isEmpty
                      ? 'Default source for music downloads'
                      : 'Default source for music downloads (not configured: '
                            '${unconfigured.join(', ')})',
                  valueListenable: SettingsManager.defaultDownloadSource,
                  items: const [
                    DropdownMenuItem(value: 'slskd', child: Text('Soulseek')),
                    DropdownMenuItem(
                      value: 'nadekodon',
                      child: Text('YouTube'),
                    ),
                  ],
                );
              },
            ),
            SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
            ListDropdown(
              title: 'Desired Audio Quality',
              subtitle: 'Quality filter for auto-download',
              valueListenable: SettingsManager.desiredAudioQuality,
              items: const [
                DropdownMenuItem(value: 'best', child: Text('Best Available')),
                DropdownMenuItem(
                  value: 'lossless',
                  child: Text('Lossless Only'),
                ),
                DropdownMenuItem(
                  value: 'high',
                  child: Text('High (≥ 320 kbps)'),
                ),
                DropdownMenuItem(
                  value: 'medium',
                  child: Text('Medium (≥ 192 kbps)'),
                ),
                DropdownMenuItem(
                  value: 'low',
                  child: Text('Low (any quality)'),
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}

/// Empty state for tabs whose every setting is admin-only.
class _AdminOnly extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Padding(
        padding: EdgeInsets.all(
          AppTheme.spaceXL * AppTheme.spaceScale(context),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.lock_outline,
              size: AppTheme.iconXL * AppTheme.iconScale(context),
              color: colors.onSurfaceVariant,
            ),
            SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
            Text('Admin access required', style: textTheme.titleMedium),
            SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
            Text(
              'Download settings are managed by an administrator.',
              style: textTheme.bodySmall?.copyWith(
                color: colors.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
