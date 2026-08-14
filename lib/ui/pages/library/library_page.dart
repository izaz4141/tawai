import 'dart:async';
import 'dart:math';
import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/services/playback_service.dart';
import 'package:tawai/services/scan_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/bridge_service.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/logger.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/widgets/mini_player.dart';
import 'package:tawai/ui/pages/library/filterable_list.dart';
import 'package:tawai/ui/pages/library/tabs/tracks_tab.dart';
import 'package:tawai/ui/pages/library/tabs/albums_tab.dart';
import 'package:tawai/ui/pages/library/tabs/artists_tab.dart';
import 'package:tawai/ui/pages/library/tabs/playlists_tab.dart';
import 'package:tawai/ui/pages/library/tabs/history_tab.dart';
import 'package:tawai/ui/widgets/components/library_search_filter.dart';
import 'package:tawai/ui/widgets/components/alphabet_index_scroller.dart';

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  final _trackScrollController = ScrollController();
  final _albumScrollController = ScrollController();
  final _artistScrollController = ScrollController();
  final _playlistScrollController = ScrollController();
  final List<GlobalKey<RefreshIndicatorState>> _refreshKeys = List.generate(
    5,
    (_) => GlobalKey<RefreshIndicatorState>(),
  );

  late final FilterableList<TrackInfo> _tracksFilterable;
  late final FilterableList<AlbumInfo> _albumsFilterable;
  late final FilterableList<ArtistInfo> _artistsFilterable;
  late final FilterableList<PlaylistInfo> _playlistsFilterable;
  late final FilterableList<PlaybackRecord> _historyFilterable;

  bool _loading = false;
  LibraryFilters _filters = LibraryFilters();
  List<String> _availableSources = [];

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 5, vsync: this);
    _tabController.addListener(() {
      if (mounted) setState(() {});
    });

    _tracksFilterable = FilterableList<TrackInfo>(
      matchesSearch: (t, q) =>
          t.title.toLowerCase().contains(q) ||
          t.artistsString.toLowerCase().contains(q) ||
          t.albumTitle.toLowerCase().contains(q),
      matchesFilters: _trackMatchesFilters,
      compare: (a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()),
    );

    _albumsFilterable = FilterableList<AlbumInfo>(
      matchesSearch: (a, q) =>
          a.title.toLowerCase().contains(q) ||
          a.artistsString.toLowerCase().contains(q),
      matchesFilters: _albumMatchesFilters,
      compare: (a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()),
    );

    _artistsFilterable = FilterableList<ArtistInfo>(
      matchesSearch: (a, q) => a.name.toLowerCase().contains(q),
      matchesFilters: _artistMatchesFilters,
      compare: (a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()),
    );

    _playlistsFilterable = FilterableList<PlaylistInfo>(
      matchesSearch: (p, q) => p.name.toLowerCase().contains(q),
      compare: (a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()),
    );

    _historyFilterable = FilterableList<PlaybackRecord>(
      matchesSearch: (r, q) =>
          r.trackTitle.toLowerCase().contains(q) ||
          r.artistName.toLowerCase().contains(q) ||
          r.albumTitle.toLowerCase().contains(q),
    );

    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _refreshKeys[_tabController.index].currentState?.show();
    });
    ScanService.instance.isScanning.addListener(_onScanStateChanged);
    SettingsManager.includedRecommendations.addListener(
      _onRecommendationSettingsChanged,
    );
    BridgeService.libraryChanged.addListener(_onLibraryChanged);
  }

  @override
  void dispose() {
    ScanService.instance.isScanning.removeListener(_onScanStateChanged);
    SettingsManager.includedRecommendations.removeListener(
      _onRecommendationSettingsChanged,
    );
    BridgeService.libraryChanged.removeListener(_onLibraryChanged);
    _tabController.dispose();
    _trackScrollController.dispose();
    _albumScrollController.dispose();
    _artistScrollController.dispose();
    _playlistScrollController.dispose();
    super.dispose();
  }

  double _albumRowHeight(BuildContext context) {
    const padding = AppTheme.spaceSM;
    const spacing = AppTheme.spaceSM;
    final crossAxisCount = AppTheme.isDesktop(context) ? 4 : 2;
    final viewportWidth = MediaQuery.of(context).size.width;
    final availableWidth =
        viewportWidth - 2 * padding - (crossAxisCount - 1) * spacing;
    final cellWidth = availableWidth / crossAxisCount;
    return cellWidth / 0.85 + spacing;
  }

  static String? _yearOf(String? date) {
    if (date == null || date.length < 4) return null;
    return int.tryParse(date.substring(0, 4))?.toString();
  }

  bool _trackMatchesFilters(TrackInfo t, LibraryFilters f) {
    if (!f.hasActiveFilters) return true;
    if (f.sources.isNotEmpty && f.sources.contains(t.source)) return true;
    if (f.genres.isNotEmpty && t.genres.any((g) => f.genres.contains(g))) {
      return true;
    }
    if (f.years.isNotEmpty && f.years.contains(_yearOf(t.releaseDate))) {
      return true;
    }
    return false;
  }

  bool _albumMatchesFilters(AlbumInfo a, LibraryFilters f) {
    if (!f.hasActiveFilters) return true;
    if (f.sources.isNotEmpty &&
        _tracksFilterable.all.any(
          (t) => t.albumId == a.id && f.sources.contains(t.source),
        )) {
      return true;
    }
    if (f.genres.isNotEmpty &&
        _tracksFilterable.all.any(
          (t) => t.albumId == a.id && t.genres.any((g) => f.genres.contains(g)),
        )) {
      return true;
    }
    if (f.years.isNotEmpty && f.years.contains(_yearOf(a.releaseDate))) {
      return true;
    }
    return false;
  }

  bool _artistMatchesFilters(ArtistInfo a, LibraryFilters f) {
    if (!f.hasActiveFilters) return true;
    final tracksOfArtist = _tracksFilterable.all.where(
      (t) => t.artists.any((ar) => ar.id == a.id),
    );
    if (f.sources.isNotEmpty &&
        tracksOfArtist.any((t) => f.sources.contains(t.source))) {
      return true;
    }
    if (f.genres.isNotEmpty &&
        tracksOfArtist.any((t) => t.genres.any((g) => f.genres.contains(g)))) {
      return true;
    }
    if (f.years.isNotEmpty &&
        tracksOfArtist.any((t) => f.years.contains(_yearOf(t.releaseDate)))) {
      return true;
    }
    return false;
  }

  LibraryFilterOptions get _filterOptions {
    final genres = <String>{};
    final years = <String>{};
    for (final t in _tracksFilterable.all) {
      genres.addAll(t.genres);
      final y = _yearOf(t.releaseDate);
      if (y != null) years.add(y);
    }
    for (final a in _albumsFilterable.all) {
      final y = _yearOf(a.releaseDate);
      if (y != null) years.add(y);
    }
    final genreList = genres.toList()..sort();
    final yearList = years.toList()..sort();
    return LibraryFilterOptions(
      sources: List.of(_availableSources),
      genres: genreList,
      years: yearList,
    );
  }

  Widget _buildScroller(
    BuildContext context,
    List<TrackInfo> tracks,
    List<AlbumInfo> albums,
    List<ArtistInfo> artists,
    List<PlaylistInfo> playlists,
  ) {
    final albumExtent = _albumRowHeight(context);
    final gridColumns = AppTheme.isDesktop(context) ? 4 : 2;
    switch (_tabController.index) {
      case 0:
        return AlphabetIndexScroller<TrackInfo>(
          scrollController: _trackScrollController,
          items: tracks,
          labelSelector: (t) => t.title,
          itemExtent: AppTheme.spaceMD * 6 * AppTheme.spaceScale(context),
        );
      case 1:
        return AlphabetIndexScroller<AlbumInfo>(
          scrollController: _albumScrollController,
          items: albums,
          labelSelector: (a) => a.title,
          itemExtent: albumExtent,
          gridColumns: gridColumns,
        );
      case 2:
        return AlphabetIndexScroller<ArtistInfo>(
          scrollController: _artistScrollController,
          items: artists,
          labelSelector: (a) => a.name,
          itemExtent: AppTheme.spaceMD * 6 * AppTheme.spaceScale(context),
        );
      case 3:
        return AlphabetIndexScroller<PlaylistInfo>(
          scrollController: _playlistScrollController,
          items: playlists,
          labelSelector: (p) => p.name,
          itemExtent: AppTheme.spaceMD * 6 * AppTheme.spaceScale(context),
          startIndex: 1,
        );
      default:
        return const SizedBox.shrink();
    }
  }

  void _onScanStateChanged() {
    if (!ScanService.instance.isScanning.value && mounted) {
      _loadAll();
    }
  }

  void _onLibraryChanged() {
    if (mounted) {
      unawaited(_loadAll());
    }
  }

  void _onRecommendationSettingsChanged() {
    if (mounted) {
      unawaited(_syncRecommendationSources());
    }
  }

  Future<void> _loadTabData<T>(
    FilterableList<T> filterable,
    Future<List<T>> Function() loader,
  ) async {
    final items = await loader();
    if (mounted) setState(() => filterable.all = items);
  }

  Future<void> _loadAll() async {
    if (_loading) return;
    _loading = true;
    await Future.wait([
      _loadTabData(_tracksFilterable, BridgeService.instance.getTracks),
      _loadTabData(_albumsFilterable, BridgeService.instance.getAlbums),
      _loadTabData(_artistsFilterable, BridgeService.instance.getArtists),
      _loadTabData(_playlistsFilterable, BridgeService.instance.getPlaylists),
      _loadTabData(_historyFilterable, () async {
        final userId = SettingsManager.currentUser.value?.id ?? '';
        return BridgeService.instance.getHistory(userId, limit: 50);
      }),
      _loadSources(),
    ]);
    await _syncRecommendationSources();
    _loading = false;
  }

  Future<void> _loadSources() async {
    final userId = SettingsManager.currentUser.value?.id ?? '';
    final sources = await BridgeService.instance.listLibrarySources(userId);
    if (mounted) {
      setState(() {
        _availableSources = sources.map((s) => s.name).toList();
      });
    }
  }

  Future<void> _syncRecommendationSources() async {
    final current = SettingsManager.includedRecommendations.value;
    final result = await BridgeService.instance.syncRecs(includedKeys: current);
    if (!result.success && result.error != null) {
      log('syncRecs failed: ${result.error}', isError: true);
    }
  }

  Future<void> _showCreatePlaylistDialog() async {
    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Create Playlist'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(hintText: 'Playlist name'),
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              final n = controller.text.trim();
              if (n.isNotEmpty) Navigator.pop(ctx, n);
            },
            child: const Text('Create'),
          ),
        ],
      ),
    );
    if (name != null && mounted) {
      await BridgeService.instance.createPlaylist(name);
      await _loadTabData(
        _playlistsFilterable,
        BridgeService.instance.getPlaylists,
      );
    }
  }

  Future<void> _deletePlaylist(String playlistId) async {
    await BridgeService.instance.deletePlaylist(playlistId);
    await _loadTabData(
      _playlistsFilterable,
      BridgeService.instance.getPlaylists,
    );
  }

  Future<void> _playHistoryEntry(PlaybackRecord r) async {
    final track = await PlaybackService.instance.fetchTrackInfo(r.trackId);
    if (track != null) {
      PlaybackService.instance.play([track]);
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final isDesktop = AppTheme.isDesktop(context);

    final filteredTracks = _tracksFilterable.filtered(_filters);
    final filteredAlbums = _albumsFilterable.filtered(_filters);
    final filteredArtists = _artistsFilterable.filtered(_filters);
    final filteredPlaylists = _playlistsFilterable.filtered(_filters);
    final filteredHistory = _historyFilterable.filtered(_filters);

    const labels = ['Songs', 'Albums', 'Artists', 'Playlists', 'History'];

    final tabBar = TabBar(
      isScrollable: true,
      controller: _tabController,
      labelStyle: textTheme.bodyMedium?.copyWith(color: colors.primary),
      unselectedLabelStyle: textTheme.bodyMedium,
      splashBorderRadius: BorderRadius.vertical(
        top: Radius.circular(AppTheme.radiusLG),
      ),
      tabs: labels.map((l) => Tab(text: l)).toList(),
    );

    return AppShell(
      title: 'Library',
      bottom: tabBar,
      floatingActionButton:
          _tabController.index == 0 && filteredTracks.isNotEmpty
          ? ValueListenableBuilder<double>(
              valueListenable: miniPlayerInset,
              builder: (context, inset, _) {
                return Padding(
                  padding: EdgeInsets.only(
                    right: AlphabetIndexScroller.kStripWidth,
                    bottom: inset,
                  ),
                  child: FloatingActionButton(
                    heroTag: 'shuffle_tracks',
                    onPressed: () async {
                      final playback = PlaybackService.instance;
                      final randomIndex = Random().nextInt(
                        filteredTracks.length,
                      );
                      final started = await playback.play(
                        filteredTracks,
                        startIndex: randomIndex,
                      );
                      if (started) {
                        playback.toggleShuffle(force: true);
                      }
                    },
                    child: const Icon(Icons.shuffle),
                  ),
                );
              },
            )
          : null,
      body: Stack(
        children: [
          Column(
            children: [
              LibrarySearchFilter(
                tabIndex: _tabController.index,
                filters: _filters,
                options: _filterOptions,
                showSearch: _tabController.index != 4,
                showFilter: _tabController.index <= 2,
                rightPadding: AlphabetIndexScroller.kStripWidth,
                onQueryChanged: (v) => setState(() => _filters.query = v),
                onFiltersChanged: (v) => setState(() => _filters = v),
              ),
              Expanded(
                child: TabBarView(
                  controller: _tabController,
                  children: [
                    RefreshIndicator(
                      key: _refreshKeys[0],
                      onRefresh: _loadAll,
                      child: LibraryTracksTab(
                        tracks: filteredTracks,
                        scrollController: _trackScrollController,
                      ),
                    ),
                    RefreshIndicator(
                      key: _refreshKeys[1],
                      onRefresh: _loadAll,
                      child: LibraryAlbumsTab(
                        albums: filteredAlbums,
                        scrollController: _albumScrollController,
                      ),
                    ),
                    RefreshIndicator(
                      key: _refreshKeys[2],
                      onRefresh: _loadAll,
                      child: LibraryArtistsTab(
                        artists: filteredArtists,
                        scrollController: _artistScrollController,
                      ),
                    ),
                    RefreshIndicator(
                      key: _refreshKeys[3],
                      onRefresh: _loadAll,
                      child: LibraryPlaylistsTab(
                        playlists: filteredPlaylists,
                        scrollController: _playlistScrollController,
                        onCreatePlaylist: _showCreatePlaylistDialog,
                        onDeletePlaylist: _deletePlaylist,
                      ),
                    ),
                    RefreshIndicator(
                      key: _refreshKeys[4],
                      onRefresh: () =>
                          _loadTabData(_historyFilterable, () async {
                            final userId =
                                SettingsManager.currentUser.value?.id ?? '';
                            return BridgeService.instance.getHistory(
                              userId,
                              limit: 50,
                            );
                          }),
                      child: LibraryHistoryTab(
                        history: filteredHistory,
                        onPlayRecord: _playHistoryEntry,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          ValueListenableBuilder<double>(
            valueListenable: miniPlayerInset,
            builder: (context, inset, _) {
              return Positioned(
                right: 0,
                top: 0,
                bottom: inset,
                width: AlphabetIndexScroller.kStripWidth,
                child: _buildScroller(
                  context,
                  filteredTracks,
                  filteredAlbums,
                  filteredArtists,
                  filteredPlaylists,
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
