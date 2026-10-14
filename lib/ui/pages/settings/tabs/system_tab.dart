import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:tawai/models/user.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_snackbar.dart';
import 'package:tawai/ui/widgets/components/list_choice.dart';
import 'package:tawai/ui/widgets/components/list_slider.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/widgets/dialog/color_picker_dialog.dart';
import 'package:tawai/utils/settings.dart';

class SettingsSystemTab extends StatefulWidget {
  const SettingsSystemTab({super.key});

  @override
  State<SettingsSystemTab> createState() => _SettingsSystemTabState();
}

class _SettingsSystemTabState extends State<SettingsSystemTab> {
  // 'lossless' parks on the extra step at the top of the range.
  static const int _minBitrate = 32;
  static const int _maxBitrate = 320;
  static const int _losslessStep = _maxBitrate + 1;

  final _obscured = ValueNotifier<bool>(true);
  late final ValueNotifier<int> _bitrateKbps;

  int _toSlider(String raw) =>
      raw == 'lossless' ? _losslessStep : (int.tryParse(raw) ?? _losslessStep);

  String _fromSlider(int kbps) =>
      kbps >= _losslessStep ? 'lossless' : kbps.toString();

  @override
  void initState() {
    super.initState();
    SettingsManager.getCurrentUserApiKey();
    _bitrateKbps = ValueNotifier(
      _toSlider(SettingsManager.preferredBitrate.value),
    );
    SettingsManager.preferredBitrate.addListener(_syncBitrate);
  }

  @override
  void dispose() {
    SettingsManager.preferredBitrate.removeListener(_syncBitrate);
    _bitrateKbps.dispose();
    _obscured.dispose();
    super.dispose();
  }

  void _syncBitrate() {
    final next = _toSlider(SettingsManager.preferredBitrate.value);
    if (_bitrateKbps.value != next) _bitrateKbps.value = next;
  }

  // Live preview while dragging; the value is only persisted once the drag
  // ends, so a single gesture does not issue hundreds of setting writes.
  void _previewBitrate(int kbps) => _bitrateKbps.value = kbps;

  void _commitBitrate(int kbps) {
    SettingsManager.saveUserSetting(
      SettingsManager.preferredBitrate,
      'preferred_bitrate',
      _fromSlider(kbps),
    );
  }

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
        ValueListenableBuilder<User?>(
          valueListenable: SettingsManager.currentUser,
          builder: (context, user, _) {
            final apiKey = user?.apiKey ?? '';
            final userId = user?.id ?? '';
            final textTheme = Theme.of(context).textTheme;
            return ValueListenableBuilder<bool>(
              valueListenable: _obscured,
              builder: (context, obscured, _) {
                return ListTile(
                  title: Text('API Key', style: textTheme.bodyMedium),
                  subtitle: Text(
                    user == null
                        ? 'No account signed in'
                        : apiKey.isEmpty
                        ? 'No API key available'
                        : obscured
                        ? List.filled(apiKey.length, '•').join()
                        : apiKey,
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.onSurfaceVariant,
                      fontFamily: 'monospace',
                    ),
                  ),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (apiKey.isNotEmpty)
                        IconButton(
                          tooltip: obscured ? 'Show API key' : 'Hide API key',
                          icon: Icon(
                            obscured ? Icons.visibility : Icons.visibility_off,
                          ),
                          iconSize:
                              AppTheme.iconSM * AppTheme.iconScale(context),
                          onPressed: () => _obscured.value = !obscured,
                        ),
                      IconButton(
                        tooltip: 'Copy API key',
                        icon: const Icon(Icons.copy),
                        iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                        onPressed: apiKey.isEmpty
                            ? null
                            : () {
                                Clipboard.setData(ClipboardData(text: apiKey));
                                AppSnackBar.show(
                                  context,
                                  'API key copied',
                                  type: SnackType.success,
                                );
                              },
                      ),
                      IconButton(
                        tooltip: 'Reroll API key',
                        icon: const Icon(Icons.refresh),
                        iconSize: AppTheme.iconSM * AppTheme.iconScale(context),
                        onPressed: userId.isEmpty
                            ? null
                            : () async {
                                final confirmed = await showDialog<bool>(
                                  context: context,
                                  builder: (ctx) => AlertDialog(
                                    title: const Text('Reroll API key?'),
                                    content: const Text(
                                      'Regenerating replaces your current API key. '
                                      'Clients using the old key will need to update.',
                                    ),
                                    actions: [
                                      TextButton(
                                        onPressed: () =>
                                            Navigator.pop(ctx, false),
                                        child: const Text('Cancel'),
                                      ),
                                      FilledButton(
                                        onPressed: () =>
                                            Navigator.pop(ctx, true),
                                        child: const Text('Reroll'),
                                      ),
                                    ],
                                  ),
                                );
                                if (confirmed != true || !context.mounted)
                                  return;
                                final newKey =
                                    await SettingsManager.regenerateApiKey(
                                      userId,
                                    );
                                if (!context.mounted) return;
                                if (newKey != null && newKey.isNotEmpty) {
                                  _obscured.value = true;
                                  AppSnackBar.show(
                                    context,
                                    'API key rerolled',
                                    type: SnackType.success,
                                  );
                                } else {
                                  AppSnackBar.show(
                                    context,
                                    'Failed to reroll API key',
                                    type: SnackType.error,
                                  );
                                }
                              },
                      ),
                    ],
                  ),
                );
              },
            );
          },
        ),
        SizedBox(height: AppTheme.spaceMD * AppTheme.spaceScale(context)),
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
        ValueListenableBuilder<int>(
          valueListenable: _bitrateKbps,
          builder: (context, kbps, _) {
            final lossless = kbps >= _losslessStep;
            return ListSlider<int>(
              title: 'Bitrate',
              subtitle: lossless
                  ? 'Lossless — playing the original file.'
                  : 'Transcoding remote streams to $kbps kbps.',
              valueListenable: _bitrateKbps,
              onChanged: _previewBitrate,
              onChangeEnd: _commitBitrate,
              min: _minBitrate,
              max: _maxBitrate,
              minValue: (label: '$_minBitrate kbps', value: _minBitrate),
              // Sentinel end: one step past the numeric max, labelled "Lossless".
              maxValue: (label: 'Lossless', value: _losslessStep),
              step: 1,
              unit: 'kbps',
              defaultValue: _losslessStep,
            );
          },
        ),
      ],
    );
  }
}
