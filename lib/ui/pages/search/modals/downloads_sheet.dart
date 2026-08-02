import 'dart:async';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/sheet.dart';
import 'package:tawai/ui/pages/search/widgets/download_tile.dart';
import 'package:tawai/utils/bridge_service.dart';

void showDownloadsSheet(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    builder: (_) => const _DownloadsSheet(),
  );
}

class _DownloadsSheet extends StatefulWidget {
  const _DownloadsSheet();

  @override
  State<_DownloadsSheet> createState() => _DownloadsSheetState();
}

class _DownloadsSheetState extends State<_DownloadsSheet> {
  List<DownloadRecord> _downloads = [];
  Timer? _pollTimer;

  @override
  void initState() {
    super.initState();
    _poll();
    _pollTimer = Timer.periodic(const Duration(seconds: 10), (_) => _poll());
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }

  Future<void> _poll() async {
    final downloads = await BridgeService.instance.pollDownloads();
    if (mounted) setState(() => _downloads = downloads);
  }

  Future<void> _cancelDownload(DownloadRecord dl) async {
    await BridgeService.instance.cancel(dl.source, dl.sourceId);
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final active = _downloads
        .where((d) => d.state == 'queued' || d.state == 'active' || d.state == 'downloading')
        .toList();
    final completed = _downloads
        .where((d) => d.state == 'completed' || d.state == 'finished')
        .toList();
    final errored = _downloads
        .where((d) => d.state == 'error' || d.state == 'errored')
        .toList();

    return Sheet(
      draggable: true,
      initialChildSize: 0.6,
      minChildSize: 0.3,
      maxChildSize: 0.9,
      header: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
        child: Text('Downloads', style: textTheme.titleMedium),
      ),
      bodyBuilder: (scrollCtrl) {
        if (_downloads.isEmpty) {
          return const Center(child: Text('No downloads yet'));
        }

        return ListView(
          controller: scrollCtrl,
          padding: EdgeInsets.only(
            left: AppTheme.spaceSM,
            right: AppTheme.spaceSM,
            bottom: AppTheme.spaceLG,
          ),
          children: [
            if (active.isNotEmpty) ...[
              Padding(
                padding: EdgeInsets.only(
                  left: AppTheme.spaceSM,
                  top: AppTheme.spaceSM,
                  bottom: AppTheme.spaceXS,
                ),
                child: Text(
                  'Active (${active.length})',
                  style: textTheme.titleSmall,
                ),
              ),
              ...active.map((dl) => DownloadTile(
                    download: dl,
                    onCancel: () => _cancelDownload(dl),
                  )),
            ],
            if (completed.isNotEmpty) ...[
              Padding(
                padding: EdgeInsets.only(
                  left: AppTheme.spaceSM,
                  top: AppTheme.spaceSM,
                  bottom: AppTheme.spaceXS,
                ),
                child: Text(
                  'Completed (${completed.length})',
                  style: textTheme.titleSmall,
                ),
              ),
              ...completed.map((dl) => DownloadTile(download: dl)),
            ],
            if (errored.isNotEmpty) ...[
              Padding(
                padding: EdgeInsets.only(
                  left: AppTheme.spaceSM,
                  top: AppTheme.spaceSM,
                  bottom: AppTheme.spaceXS,
                ),
                child: Text(
                  'Failed (${errored.length})',
                  style: textTheme.titleSmall,
                ),
              ),
              ...errored.map((dl) => DownloadTile(download: dl)),
            ],
          ],
        );
      },
    );
  }
}
