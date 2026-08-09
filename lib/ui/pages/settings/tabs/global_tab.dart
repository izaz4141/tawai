import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/api_service.dart';
import 'package:tawai/ui/pages/settings/modals/connect_service.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/system_service.dart';
import 'package:tawai/ui/widgets/components/list_button.dart';
import 'package:tawai/ui/widgets/components/list_dropdown.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/list_text_field.dart';
import 'package:tawai/ui/widgets/components/spin_box.dart';
import 'package:tawai/ui/widgets/dialog/db_account_manager_dialog.dart';
import 'package:tawai/ui/widgets/dialog/permission_dialog.dart';
import 'package:tawai/utils/io_service.dart';
import 'package:tawai/utils/platform_service.dart';

class SettingsGlobalTab extends StatelessWidget {
  const SettingsGlobalTab({super.key});

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        SectionHeader(
          title: 'Server',
          leading: const Icon(Icons.dns),
          trailing: ValueListenableBuilder<bool>(
            valueListenable: APIService.isOnline,
            builder: (context, online, _) {
              final colors = Theme.of(context).colorScheme;
              return IconButton(
                tooltip: 'Restart server',
                onPressed: () => SettingsManager.restartServer(),
                icon: Icon(
                  Icons.restart_alt,
                  color: online ? colors.primary : colors.error,
                ),
              );
            },
          ),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListTextField(
          title: 'Server Host',
          subtitle: 'Hostname or IP address of the server',
          valueListenable: SettingsManager.serverHost,
          defaultValue: SettingsManager.defaults['server_host'] as String?,
        ),
        SpinBox<int>(
          title: 'Server Port',
          subtitle: 'Port the server listens on',
          valueListenable: SettingsManager.serverPort,
          min: 1,
          max: 65535,
          defaultValue: SettingsManager.defaults['server_port'] as int?,
        ),
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Require Login',
          subtitle: 'Enforce authentication for remote access',
          valueListenable: SettingsManager.requireLogin,
          defaultValue: SettingsManager.defaults['require_login'] as bool?,
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        OutlinedButton.icon(
          onPressed: () {
            showDialog(
              context: context,
              builder: (ctx) => const DbAccountManagerDialog(),
            );
          },
          icon: Icon(
            Icons.manage_accounts,
            size: AppTheme.iconMD * AppTheme.iconScale(context),
          ),
          label: const Text('Manage Accounts'),
        ),
        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
        SectionHeader(title: 'Downloads', leading: const Icon(Icons.download)),
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
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        ListenableBuilder(
          listenable: Listenable.merge([
            SystemService().slskdVersion,
            SystemService().nadekodonVersion,
          ]),
          builder: (context, _) {
            final sys = SystemService();
            final disabled = <String>{};
            if (sys.slskdVersion.value == null) disabled.add('slskd');
            if (sys.nadekodonVersion.value == null) disabled.add('nadekodon');
            return ListDropdown(
              title: 'Download Source',
              subtitle: 'Default source for music downloads',
              valueListenable: SettingsManager.defaultDownloadSource,
              disabledValues: disabled,
              items: const [
                DropdownMenuItem(value: 'slskd', child: Text('Soulseek')),
                DropdownMenuItem(value: 'nadekodon', child: Text('YouTube')),
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
            DropdownMenuItem(value: 'lossless', child: Text('Lossless Only')),
            DropdownMenuItem(value: 'high', child: Text('High (≥ 320 kbps)')),
            DropdownMenuItem(
              value: 'medium',
              child: Text('Medium (≥ 192 kbps)'),
            ),
            DropdownMenuItem(value: 'low', child: Text('Low (any quality)')),
          ],
        ),
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        if (PlatformService.isAndroid) ...[
          ListButton(
            title: 'App Permissions',
            subtitle: 'Storage, notification and background access',
            leading: Icon(Icons.shield_outlined),
            onPressed: () => showDialog(
              context: context,
              builder: (ctx) => const PermissionDialog(),
            ),
            type: ListButtonType.text,
            label: 'Manage',
          ),
          SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        ],
        ListButton(
          title: 'slskd',
          subtitle: 'Configure slskd connection',
          onPressed: () => showConnectServiceDialog(
            context: context,
            inputs: [
              (
                'slskd URL',
                'Your slskd server URL',
                false,
                SettingsManager.slskdUrl.value,
              ),
              (
                'slskd API Key',
                'Stored encrypted server-side; blank keeps the existing key',
                true,
                '',
              ),
            ],
            test: (values) async {
              final res = await BridgeService.instance.testConnection('slskd');
              return res.success
                  ? 'Connected! v${res.version ?? '?'}'
                  : res.error ?? 'Connection failed';
            },
            onConfirm: (values) async {
              SettingsManager.slskdUrl.value = values[0];
              await SettingsManager.saveSlskdApiKey(values[1]);
              SystemService().checkServiceAvailability();
            },
          ),
          type: ListButtonType.text,
          label: 'Configure',
        ),
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        ListButton(
          title: 'Nadekodon (YouTube)',
          subtitle: 'Configure nadekodon connection',
          onPressed: () => showConnectServiceDialog(
            context: context,
            inputs: [
              (
                'Nadekodon URL',
                'Your nadekodon server URL',
                false,
                SettingsManager.nadekodonUrl.value,
              ),
              (
                'Nadekodon API Key',
                'Stored encrypted server-side; blank keeps the existing key',
                true,
                '',
              ),
            ],
            test: (values) async {
              final res = await BridgeService.instance.testConnection(
                'nadekodon',
              );
              return res.success
                  ? 'Connected! v${res.version ?? '?'}'
                  : res.error ?? 'Connection failed';
            },
            onConfirm: (values) async {
              SettingsManager.nadekodonUrl.value = values[0];
              await SettingsManager.saveNadekodonApiKey(values[1]);
              SystemService().checkServiceAvailability();
            },
          ),
          type: ListButtonType.text,
          label: 'Configure',
        ),
      ],
    );
  }
}
