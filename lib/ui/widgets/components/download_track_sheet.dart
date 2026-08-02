import 'package:flutter/material.dart';

import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/widgets/components/cover_image.dart';
import 'package:tawai/ui/pages/search/search_types.dart';
import 'package:tawai/ui/pages/search/widgets/search_result_tile.dart';
import 'package:tawai/ui/pages/search/modals/nadekodon_format_picker.dart';

void showTrackDownloadSheet(BuildContext context, TrackInfo track) {
  showModalBottomSheet(
    context: context,
    builder: (_) => _DownloadTrackSheet(track: track),
  );
}

class _DownloadTrackSheet extends StatefulWidget {
  final TrackInfo track;

  const _DownloadTrackSheet({required this.track});

  @override
  State<_DownloadTrackSheet> createState() => _DownloadTrackSheetState();
}

class _DownloadTrackSheetState extends State<_DownloadTrackSheet> {
  List<SearchResultItem> _results = [];
  bool _searching = true;
  String? _error;
  final _loadingUrls = <String>{};
  late final String _source;

  @override
  void initState() {
    super.initState();
    _source = SettingsManager.defaultDownloadSource.value;
    _search();
  }

  Future<void> _search() async {
    setState(() {
      _searching = true;
      _error = null;
      _results = [];
    });

    try {
      final query = '${widget.track.title} ${widget.track.artistsString}';
      final results = await BridgeService.instance.search(_source, query);
      if (mounted) {
        setState(() {
          _results = results;
          _searching = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _searching = false;
        });
      }
    }
  }

  Future<void> _downloadSlskd(SearchResultItem entry) async {
    final userId = SettingsManager.currentUserId.value ?? '';
    if (userId.isEmpty) {
      _showError('No user configured');
      return;
    }

    final result = await BridgeService.instance.create(
      'slskd',
      '${entry.username}/${entry.filename}',
      SettingsManager.downloadFolder.value,
      userId,
      extra: '{"username": "${entry.username}"}',
    );

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            result.success
                ? 'Downloading: ${entry.filename.split('/').last}'
                : result.error ?? 'Download failed',
          ),
        ),
      );
    }
  }

  Future<void> _downloadNadekodon(SearchResultItem entry) async {
    setState(() => _loadingUrls.add(entry.filename));

    try {
      final infoJson = await BridgeService.instance.getInfo(
        'nadekodon',
        entry.webpageUrl ?? entry.filename,
      );

      if (!mounted) return;

      if (infoJson == null) {
        _showError('Failed to get info for ${entry.title ?? entry.filename}');
        return;
      }

      final formatId = await showNadekodonFormatPicker(
        context,
        infoJson: infoJson,
      );

      if (!mounted || formatId == null) return;

      final userId = SettingsManager.currentUserId.value ?? '';
      if (userId.isEmpty) {
        _showError('No user configured');
        return;
      }

      final result = await BridgeService.instance.create(
        'nadekodon',
        entry.filename,
        SettingsManager.downloadFolder.value,
        userId,
        extra: '{"audio_format": "$formatId"}',
      );

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              result.success
                  ? 'Downloading: ${entry.title ?? entry.filename}'
                  : result.error ?? 'Download failed',
            ),
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _loadingUrls.remove(entry.filename));
    }
  }

  void _showError(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
  }

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
                child: CoverImage(
                  trackId: widget.track.id,
                  iconSize: AppTheme.iconMD,
                ),
              ),
            ),
            const SizedBox(width: AppTheme.spaceMD),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text('Download', style: theme.textTheme.titleMedium),
                  Text(
                    '${widget.track.title} via $_source',
                    style: theme.textTheme.bodySmall,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_searching) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: AppTheme.spaceXXL),
        child: const Center(child: CircularProgressIndicator()),
      );
    }

    if (_error != null) {
      return Padding(
        padding: const EdgeInsets.symmetric(
          vertical: AppTheme.spaceXXL,
          horizontal: AppTheme.spaceXL,
        ),
        child: Column(
          children: [
            const Icon(Icons.error_outline, size: AppTheme.iconXL),
            const SizedBox(height: AppTheme.spaceMD),
            Text(
              'Search failed',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: AppTheme.spaceXS),
            Text(
              _error!,
              style: Theme.of(context).textTheme.bodySmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: AppTheme.spaceXL),
            FilledButton.tonalIcon(
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
              onPressed: _search,
            ),
          ],
        ),
      );
    }

    if (_results.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: AppTheme.spaceXXL),
        child: Column(
          children: [
            const Icon(Icons.search_off, size: AppTheme.iconXL),
            const SizedBox(height: AppTheme.spaceMD),
            Text(
              'No results found',
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      itemCount: _results.length,
      shrinkWrap: true,
      itemBuilder: (context, index) {
        final entry = _results[index];
        return SearchResultTile(
          entry: entry,
          loadingUrls: _loadingUrls,
          onDownload: () {
            if (entry.sourceType == 'slskd') {
              _downloadSlskd(entry);
            } else {
              _downloadNadekodon(entry);
            }
          },
        );
      },
    );
  }
}
