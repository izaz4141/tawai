import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_slider.dart';
import 'package:tawai/ui/widgets/components/list_switch.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/utils/settings.dart';

class SettingsPlaybackTab extends StatefulWidget {
  const SettingsPlaybackTab({super.key});

  @override
  State<SettingsPlaybackTab> createState() => _SettingsPlaybackTabState();
}

class _SettingsPlaybackTabState extends State<SettingsPlaybackTab> {
  // 'lossless' parks on the extra step at the top of the range.
  static const int _minBitrate = 32;
  static const int _maxBitrate = 320;
  static const int _losslessStep = _maxBitrate + 1;

  late final ValueNotifier<int> _bitrateKbps;
  late final ValueNotifier<double> _preamp;

  int _toSlider(String raw) =>
      raw == 'lossless' ? _losslessStep : (int.tryParse(raw) ?? _losslessStep);

  String _fromSlider(int kbps) =>
      kbps >= _losslessStep ? 'lossless' : kbps.toString();

  @override
  void initState() {
    super.initState();
    _bitrateKbps = ValueNotifier(
      _toSlider(SettingsManager.preferredBitrate.value),
    );
    SettingsManager.preferredBitrate.addListener(_syncBitrate);

    _preamp = ValueNotifier(SettingsManager.replayGainPreamp.value);
    SettingsManager.replayGainPreamp.addListener(_syncPreamp);
  }

  @override
  void dispose() {
    SettingsManager.preferredBitrate.removeListener(_syncBitrate);
    SettingsManager.replayGainPreamp.removeListener(_syncPreamp);
    _bitrateKbps.dispose();
    _preamp.dispose();
    super.dispose();
  }

  void _syncBitrate() {
    final next = _toSlider(SettingsManager.preferredBitrate.value);
    if (_bitrateKbps.value != next) _bitrateKbps.value = next;
  }

  void _syncPreamp() {
    final next = SettingsManager.replayGainPreamp.value;
    if (_preamp.value != next) _preamp.value = next;
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

  void _previewPreamp(double value) => _preamp.value = value;

  void _commitPreamp(double value) {
    SettingsManager.saveUserSetting(
      SettingsManager.replayGainPreamp,
      'replay_gain_preamp',
      value,
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: EdgeInsets.symmetric(
        horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        vertical: AppTheme.spaceLG,
      ),
      children: [
        const SectionHeader(
          title: 'Streaming',
          leading: Icon(Icons.headphones),
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
        SizedBox(height: AppTheme.spaceXL * AppTheme.spaceScale(context)),
        const SectionHeader(
          title: 'ReplayGain',
          leading: Icon(Icons.graphic_eq),
        ),
        SizedBox(height: AppTheme.spaceSM * AppTheme.spaceScale(context)),
        ListSwitch(
          title: 'Enable ReplayGain',
          subtitle: 'Even out loudness between tracks',
          valueListenable: SettingsManager.replayGainEnabled,
          onChanged: (value) => SettingsManager.saveUserSetting(
            SettingsManager.replayGainEnabled,
            'replay_gain_enabled',
            value,
          ),
        ),
        ValueListenableBuilder<bool>(
          valueListenable: SettingsManager.replayGainEnabled,
          builder: (context, enabled, _) {
            if (!enabled) return const SizedBox.shrink();
            return Column(
              children: [
                SizedBox(
                  height: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                ListSlider<double>(
                  title: 'Preamp',
                  subtitle: 'Extra gain applied on top of each track\'s value.',
                  valueListenable: _preamp,
                  onChanged: _previewPreamp,
                  onChangeEnd: _commitPreamp,
                  min: -12,
                  max: 12,
                  step: 0.5,
                  unit: 'dB',
                  defaultValue: 0.0,
                  formatValue: (value) => '${value.toStringAsFixed(1)} dB',
                ),
              ],
            );
          },
        ),
      ],
    );
  }
}
