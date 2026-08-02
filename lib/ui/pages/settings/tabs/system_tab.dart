import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_choice.dart';
import 'package:tawai/ui/widgets/components/list_dropdown.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/dialog/color_picker_dialog.dart';
import 'package:tawai/utils/settings.dart';

class SettingsSystemTab extends StatelessWidget {
  const SettingsSystemTab({super.key});

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        SectionHeader(title: 'General', leading: const Icon(Icons.settings)),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Retreat to Tray',
          subtitle: 'Minimize to system tray instead of quitting',
          valueListenable: SettingsManager.retreatToTray,
          defaultValue: SettingsManager.defaults['retreat_to_tray'] as bool?,
        ),
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Nightly Updates',
          subtitle: 'Check for pre-release updates',
          valueListenable: SettingsManager.checkNightly,
          defaultValue: SettingsManager.defaults['check_nightly'] as bool?,
        ),
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
        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
        SectionHeader(title: 'Appearance', leading: const Icon(Icons.palette)),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListChoice<ThemeMode>(
          title: 'Theme Mode',
          subtitle: 'Choose your preferred theme',
          valueListenable: SettingsManager.themeMode,
          items: const [
            (ThemeMode.system, 'System', Icons.settings_suggest_rounded),
            (ThemeMode.light, 'Light', Icons.light_mode),
            (ThemeMode.dark, 'Dark', Icons.dark_mode),
          ],
        ),
        SizedBox(height: AppTheme.spaceLG * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Dynamic Color',
          subtitle: 'Use system accent color',
          valueListenable: SettingsManager.useDynamicColor,
          defaultValue: SettingsManager.defaults['use_dynamic_color'] as bool?,
        ),
        ValueListenableBuilder<bool>(
          valueListenable: SettingsManager.useDynamicColor,
          builder: (context, useDynamic, _) {
            if (useDynamic) return const SizedBox.shrink();
            return Column(
              children: [
                SizedBox(
                  height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                ValueListenableBuilder<int>(
                  valueListenable: SettingsManager.customColor,
                  builder: (context, value, _) {
                    final color = Color(value);
                    return ListTile(
                      title: const Text('Custom Color'),
                      subtitle: Text(
                        '#${color.toARGB32().toRadixString(16).padLeft(8, '0')}',
                      ),
                      trailing: Container(
                        width: AppTheme.iconLG * AppTheme.iconScale(context),
                        height: AppTheme.iconLG * AppTheme.iconScale(context),
                        decoration: BoxDecoration(
                          color: color,
                          borderRadius: BorderRadius.circular(
                            AppTheme.radiusSM * AppTheme.radiusScale(context),
                          ),
                          border: Border.all(color: colors.outlineVariant),
                        ),
                      ),
                      onTap: () async {
                        final picked = await showDialog<Color>(
                          context: context,
                          builder: (context) =>
                              ColorPickerDialog(initial: color),
                        );
                        if (picked != null) {
                          SettingsManager.customColor.value = picked.toARGB32();
                        }
                      },
                    );
                  },
                ),
              ],
            );
          },
        ),
        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
        SectionHeader(
          title: 'Streaming',
          leading: const Icon(Icons.headphones),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListDropdown(
          title: 'Preferred Bitrate',
          subtitle:
              'Transcode remote streams to this bitrate. '
              'Lossless streams the original file.',
          valueListenable: SettingsManager.preferredBitrate,
          items: const [
            DropdownMenuItem(
              value: 'lossless',
              child: Text('Lossless (Original)'),
            ),
            DropdownMenuItem(value: '320', child: Text('320 kbps')),
            DropdownMenuItem(value: '256', child: Text('256 kbps')),
            DropdownMenuItem(value: '192', child: Text('192 kbps')),
            DropdownMenuItem(value: '128', child: Text('128 kbps')),
          ],
          onChange: (v) => SettingsManager.saveUserSetting(SettingsManager.preferredBitrate, 'preferred_bitrate', v),
        ),
      ],
    );
  }
}
