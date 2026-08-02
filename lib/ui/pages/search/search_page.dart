import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/pages/search/search_types.dart';
import 'package:tawai/ui/pages/search/widgets/search_bar.dart' as search;
import 'package:tawai/ui/pages/search/widgets/search_result_tile.dart';
import 'package:tawai/ui/pages/search/widgets/enhanced_search_view.dart';
import 'package:tawai/ui/pages/search/modals/search_settings_sheet.dart';
import 'package:tawai/ui/pages/search/modals/downloads_sheet.dart';
import 'package:tawai/ui/pages/search/modals/nadekodon_format_picker.dart';
import 'package:tawai/ui/pages/search/utils/helper.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _controller = TextEditingController();
  int _activeCount = 0;
  Timer? _badgeTimer;
  bool _autoDownloadEnabled = false;
  bool _enhancedSearch = true;
  final _loadingUrls = <String>{};
  late String _downloadSource;
  late SearchMode _searchMode;

  List<SearchResultItem> _results = [];
  bool _searching = false;
  bool _autoDownloading = false;

  List<RecordingInfo> _recordings = [];
  bool _searchingEnhanced = false;

  @override
  void initState() {
    super.initState();
    _downloadSource = SettingsManager.defaultDownloadSource.value;
    _searchMode = _enhancedSearch ? SearchMode.enhanced : SearchMode.standard;
    _pollBadge();
    _badgeTimer = Timer.periodic(
      const Duration(seconds: 30),
      (_) => _pollBadge(),
    );
  }

  @override
  void dispose() {
    _badgeTimer?.cancel();
    _controller.dispose();
    super.dispose();
  }

  Future<void> _pollBadge() async {
    final downloads = await BridgeService.instance.listDownloads();
    if (mounted) {
      setState(() {
        _activeCount = downloads
            .where((d) => d.state == 'active' || d.state == 'downloading')
            .length;
      });
    }
  }

  Future<void> _search() async {
    final query = _controller.text.trim();
    if (query.isEmpty) return;

    if (_searchMode == SearchMode.enhanced) {
      setState(() {
        _searchingEnhanced = true;
        _recordings = [];
        _results = [];
      });
      final recordings = await BridgeService.instance.searchMusicBrainz(query);
      if (mounted) {
        setState(() {
          _recordings = recordings;
          _searchingEnhanced = false;
        });
      }
    } else {
      setState(() => _searching = true);
      final results = await _doSearch(query);
      if (mounted) {
        setState(() {
          _results = results;
          _searching = false;
        });
      }
    }
  }

  Future<List<SearchResultItem>> _doSearch(String query) async {
    if (query.isEmpty) return [];
    return BridgeService.instance.search(_downloadSource, query);
  }

  Future<void> _autoDownload() async {
    final query = _controller.text.trim();
    if (query.isEmpty) return;

    setState(() => _autoDownloading = true);
    final results = await _doSearch(query);

    if (results.isEmpty) {
      if (mounted) {
        setState(() => _autoDownloading = false);
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('No results for "$query"')));
      }
      return;
    }

    if (_downloadSource == 'slskd') {
      final quality = SettingsManager.desiredAudioQuality.value;
      final best = pickBestMatch(results, quality);

      if (best == null) {
        if (mounted) {
          setState(() => _autoDownloading = false);
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                'No results match quality "$quality" for "$query"',
              ),
            ),
          );
        }
        return;
      }

      final userId = SettingsManager.currentUserId.value ?? '';
      await BridgeService.instance.create(
        'slskd',
        '${best.username}/${best.filename}',
        SettingsManager.downloadFolder.value,
        userId,
        extra: '{"username": "${best.username}"}',
      );

      if (mounted) {
        setState(() => _autoDownloading = false);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              'Auto-downloading: ${best.filename.split('/').last}',
            ),
          ),
        );
      }
    } else {
      final first = results.firstOrNull;
      if (first == null) {
        if (mounted) {
          setState(() => _autoDownloading = false);
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(SnackBar(content: Text('No results for "$query"')));
        }
        return;
      }
      await _downloadNadekodon(first);
      if (mounted) setState(() => _autoDownloading = false);
    }
  }

  Future<void> _downloadSlskd(SearchResultItem entry) async {
    final userId = SettingsManager.currentUserId.value ?? '';
    await BridgeService.instance.create(
      'slskd',
      '${entry.username}/${entry.filename}',
      SettingsManager.downloadFolder.value,
      userId,
      extra: '{"username": "${entry.username}"}',
    );
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Downloading: ${entry.filename}')),
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
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to get info for ${entry.title ?? entry.filename}'),
          ),
        );
        return;
      }

      String? formatId;
      if (_autoDownloadEnabled) {
        final data = jsonDecode(infoJson) as Map<String, dynamic>;
        final items = data['items'] as List<dynamic>;
        final audioFormats = <Map<String, dynamic>>[];
        for (final item in items) {
          final audios = (item['audios'] as List<dynamic>?) ?? [];
          for (final f in audios) {
            final m = f as Map<String, dynamic>;
            if (m['acodec'] != null && m['acodec'] != 'none') {
              audioFormats.add(m);
            }
          }
        }
        final quality = SettingsManager.desiredAudioQuality.value;
        formatId = pickNadekodonFormat(audioFormats, quality);
        if (formatId == null && mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                'No formats match quality "$quality" for ${entry.title ?? entry.filename}',
              ),
            ),
          );
        }
      } else {
        setState(() => _loadingUrls.remove(entry.filename));
        formatId = await showNadekodonFormatPicker(
          context,
          infoJson: infoJson,
        );
        if (!mounted) return;
        if (formatId == null) return;
      }

      if (formatId == null) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                'No formats available for ${entry.title ?? entry.filename}',
              ),
            ),
          );
        }
        return;
      }

      final userId = SettingsManager.currentUserId.value ?? '';
      await BridgeService.instance.create(
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
              'Downloading: ${entry.title ?? entry.filename}',
            ),
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _loadingUrls.remove(entry.filename));
    }
  }

  void _openSettings() {
    showSearchSettingsSheet(
      context: context,
      downloadSource: _downloadSource,
      autoDownload: _autoDownloadEnabled,
      enhancedSearch: _enhancedSearch,
      onDownloadSourceChanged: (v) {
        setState(() => _downloadSource = v);
        SettingsManager.defaultDownloadSource.value = v;
      },
      onAutoDownloadChanged: (v) => setState(() => _autoDownloadEnabled = v),
      onEnhancedSearchChanged: (v) => setState(() {
        _enhancedSearch = v;
        _searchMode = v ? SearchMode.enhanced : SearchMode.standard;
        _results = [];
        _recordings = [];
      }),
    );
  }

  void _openDownloads() {
    showDownloadsSheet(context);
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final hintText = _downloadSource == 'slskd'
        ? 'Search Soulseek...'
        : 'Search YouTube...';

    return AppShell(
      title: 'Search',
      body: Column(
        children: [
          Padding(
            padding: EdgeInsets.symmetric(
              horizontal: AppTheme.spaceSM,
              vertical: AppTheme.spaceXS,
            ),
            child: Row(
              children: [
                Expanded(
                  child: search.SearchInputBar(
                    controller: _controller,
                    hintText: _searchMode == SearchMode.enhanced
                        ? 'Search MusicBrainz...'
                        : hintText,
                    mode: _searchMode,
                    isSearching: _searchMode == SearchMode.enhanced
                        ? _searchingEnhanced
                        : _searching,
                    showAutoDownload:
                        _autoDownloadEnabled &&
                        _downloadSource == 'slskd' &&
                        _searchMode == SearchMode.standard,
                    isAutoDownloading: _autoDownloading,
                    onSearch: _search,
                    onAutoDownload: () => _autoDownload(),
                  ),
                ),
                _buildDownloadIcon(colors),
                IconButton(
                  icon: const Icon(Icons.settings),
                  tooltip: 'Search settings',
                  onPressed: _openSettings,
                ),
              ],
            ),
          ),
          Expanded(
            child: _searchMode == SearchMode.standard
                ? _buildStandardView()
                : EnhancedSearchView(
                    downloadSource: _downloadSource,
                    recordings: _recordings,
                    searching: _searchingEnhanced,
                    loadingUrls: _loadingUrls,
                    onSearch: _doSearch,
                    onDownload: (entry) {
                      if (entry.sourceType == 'slskd') {
                        _downloadSlskd(entry);
                      } else {
                        _downloadNadekodon(entry);
                      }
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildDownloadIcon(ColorScheme colors) {
    return Stack(
      children: [
        IconButton(
          icon: const Icon(Icons.download),
          tooltip: 'Downloads',
          onPressed: _openDownloads,
        ),
        if (_activeCount > 0)
          Positioned(
            right: 6,
            top: 6,
            child: Container(
              padding: EdgeInsets.all(AppTheme.spaceXS / 2),
              decoration: BoxDecoration(
                color: colors.primary,
                shape: BoxShape.circle,
              ),
              constraints: BoxConstraints(
                minWidth: AppTheme.iconSM,
                minHeight: AppTheme.iconSM,
              ),
              child: Text(
                '$_activeCount',
                style: TextStyle(
                  color: colors.onPrimary,
                  fontSize: AppTheme.textXS,
                  fontWeight: FontWeight.bold,
                ),
                textAlign: TextAlign.center,
              ),
            ),
          ),
      ],
    );
  }

  Widget _buildStandardView() {
    return _searching
        ? const Center(child: CircularProgressIndicator())
        : _buildResultList(_results);
  }

  Widget _buildResultList(List<SearchResultItem> entries) {
    if (entries.isEmpty) {
      return const Center(child: Text('No results. Try searching above.'));
    }
    return ListView.builder(
      itemCount: entries.length,
      itemBuilder: (context, index) {
        final entry = entries[index];
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
