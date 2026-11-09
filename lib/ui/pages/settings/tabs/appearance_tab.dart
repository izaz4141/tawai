import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_choice.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/dialog/color_picker_dialog.dart';
import 'package:tawai/utils/settings.dart';

class SettingsAppearanceTab extends StatefulWidget {
  const SettingsAppearanceTab({super.key});

  @override
  State<SettingsAppearanceTab> createState() => _SettingsAppearanceTabState();
}

class _SettingsAppearanceTabState extends State<SettingsAppearanceTab> {
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        const SectionHeader(title: 'Theme', leading: Icon(Icons.palette)),
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
          defaultValue:
              SettingsManager.defaultFor('interface.theme.dynamic_color')
                  as bool?,
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
                        '#${(color.toARGB32() & 0xFFFFFF).toRadixString(16).padLeft(6, '0').toUpperCase()}',
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
        // A browser has no system tray, so the switch is hidden there.
        if (!kIsWeb) ...[
          SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
          const SectionHeader(
            title: 'Window',
            leading: Icon(Icons.window_outlined),
          ),
          SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
          ListSwitch(
            title: 'Retreat to Tray',
            subtitle: 'Minimize to system tray instead of quitting',
            valueListenable: SettingsManager.retreatToTray,
            defaultValue:
                SettingsManager.defaultFor('interface.window.retreat_to_tray')
                    as bool?,
          ),
        ],
      ],
    );
  }
}
