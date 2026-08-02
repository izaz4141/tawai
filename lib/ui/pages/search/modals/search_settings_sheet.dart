import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/list_choice.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/utils/system_service.dart';

void showSearchSettingsSheet({
  required BuildContext context,
  required String downloadSource,
  required bool autoDownload,
  required bool enhancedSearch,
  required ValueChanged<String> onDownloadSourceChanged,
  required ValueChanged<bool> onAutoDownloadChanged,
  required ValueChanged<bool> onEnhancedSearchChanged,
}) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _SearchSettingsSheet(
      downloadSource: downloadSource,
      autoDownload: autoDownload,
      enhancedSearch: enhancedSearch,
      onDownloadSourceChanged: onDownloadSourceChanged,
      onAutoDownloadChanged: onAutoDownloadChanged,
      onEnhancedDownloadChanged: onEnhancedSearchChanged,
    ),
  );
}

class _SearchSettingsSheet extends StatefulWidget {
  final String downloadSource;
  final bool autoDownload;
  final bool enhancedSearch;
  final ValueChanged<String> onDownloadSourceChanged;
  final ValueChanged<bool> onAutoDownloadChanged;
  final ValueChanged<bool> onEnhancedDownloadChanged;

  const _SearchSettingsSheet({
    required this.downloadSource,
    required this.autoDownload,
    required this.enhancedSearch,
    required this.onDownloadSourceChanged,
    required this.onAutoDownloadChanged,
    required this.onEnhancedDownloadChanged,
  });

  @override
  State<_SearchSettingsSheet> createState() => _SearchSettingsSheetState();
}

class _SearchSettingsSheetState extends State<_SearchSettingsSheet> {
  late ValueNotifier<String> _downloadSourceNotifier;
  late bool _autoDownload;
  late bool _enhancedSearch;

  @override
  void initState() {
    super.initState();
    final current = widget.downloadSource;
    final sys = SystemService();
    if (current == 'nadekodon' && sys.nadekodonVersion.value == null) {
      widget.onDownloadSourceChanged('slskd');
    } else if (current == 'slskd' && sys.slskdVersion.value == null) {
      widget.onDownloadSourceChanged('nadekodon');
    }
    _downloadSourceNotifier = ValueNotifier<String>(widget.downloadSource);
    _autoDownload = widget.autoDownload;
    _enhancedSearch = widget.enhancedSearch;
  }

  @override
  void dispose() {
    _downloadSourceNotifier.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;

    return Sheet(
      header: Padding(
        padding: EdgeInsets.fromLTRB(
          AppTheme.spaceMD,
          AppTheme.spaceSM,
          AppTheme.spaceMD,
          AppTheme.spaceXS,
        ),
        child: Text('Search Settings', style: textTheme.titleMedium),
      ),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Divider(height: 1),
          ListenableBuilder(
            listenable: Listenable.merge([
              SystemService().slskdVersion,
              SystemService().nadekodonVersion,
            ]),
            builder: (context, _) {
              final sys = SystemService();
              final items = <(String, String, IconData?)>[
                if (sys.slskdVersion.value != null)
                  ('slskd', 'Soulseek', Icons.cloud),
                if (sys.nadekodonVersion.value != null)
                  ('nadekodon', 'YouTube', Icons.video_library),
              ];
              if (items.isEmpty) {
                return ListTile(
                  title: Text(
                    'No download sources available',
                    style: textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                );
              }
              return ListChoice<String>(
                title: 'Downloader',
                subtitle: '',
                valueListenable: _downloadSourceNotifier,
                items: items,
                onChanged: (v) {
                  _downloadSourceNotifier.value = v;
                  widget.onDownloadSourceChanged(v);
                },
              );
            },
          ),
          SwitchListTile(
            title: Text('Auto-download best match', style: textTheme.bodyMedium),
            subtitle: Text(
              'Show quick-download bolt button',
              style: textTheme.bodySmall,
            ),
            value: _autoDownload,
            onChanged: (v) {
              setState(() => _autoDownload = v);
              widget.onAutoDownloadChanged(v);
            },
          ),
          SwitchListTile(
            title: Text('Enhanced search by default', style: textTheme.bodyMedium),
            subtitle: Text(
              'Start in MusicBrainz discovery mode',
              style: textTheme.bodySmall,
            ),
            value: _enhancedSearch,
            onChanged: (v) {
              setState(() => _enhancedSearch = v);
              widget.onEnhancedDownloadChanged(v);
            },
          ),
        ],
      ),
    );
  }
}
