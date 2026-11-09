import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/dialog/permission_dialog.dart';
import 'package:tawai/ui/widgets/components/list_button.dart';
import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/utils/settings.dart';

class SettingsSystemTab extends StatefulWidget {
  const SettingsSystemTab({super.key});

  @override
  State<SettingsSystemTab> createState() => _SettingsSystemTabState();
}

class _SettingsSystemTabState extends State<SettingsSystemTab> {
  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        const SectionHeader(title: 'Updates', leading: Icon(Icons.update)),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Nightly Updates',
          subtitle: 'Check for pre-release updates',
          valueListenable: SettingsManager.checkNightly,
          defaultValue:
              SettingsManager.defaultFor('system.check_nightly') as bool?,
        ),
        // The database lives on the server's disk, not in the browser.
        if (!kIsWeb) ...[
          SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
          FutureBuilder<String>(
            future: SettingsManager.getDatabasePath(),
            builder: (context, snapshot) {
              return TextField(
                decoration: InputDecoration(
                  labelText: 'Database Path',
                  helperText: 'Location of the local SQLite database',
                  border: const OutlineInputBorder(),
                  enabled: false,
                ),
                controller: TextEditingController.fromValue(
                  TextEditingValue(text: snapshot.data ?? 'Loading...'),
                ),
              );
            },
          ),
        ],
        if (PlatformService.isAndroid) ...[
          SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
          const SectionHeader(
            title: 'Permissions',
            leading: Icon(Icons.shield_outlined),
          ),
          SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          ListButton(
            title: 'App Permissions',
            subtitle: 'Storage, notification and background access',
            leading: const Icon(Icons.shield_outlined),
            onPressed: () => showDialog(
              context: context,
              builder: (ctx) => const PermissionDialog(),
            ),
            type: ListButtonType.text,
            label: 'Manage',
          ),
        ],
      ],
    );
  }
}
