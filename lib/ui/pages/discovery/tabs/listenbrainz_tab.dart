import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/discovery/widgets/recommendation_section.dart';
import 'package:tawai/utils/bridge_service.dart';

class _SectionData {
  List<DiscoveryRecording>? recordings;
  bool loading = true;
}

class _ExplorationData extends _SectionData {
  int index = 0;
  int playlistCount = 0;
  String? playlistTitle;
  bool fetching = false;
}

class ListenBrainzTab extends StatefulWidget {
  final Map<
    String,
    ({List<DiscoveryRecording> recordings, String? title, int count})
  >
  lbPlaylistCache;
  final Map<int, String> lbExplorationIndexMap;

  const ListenBrainzTab({
    super.key,
    required this.lbPlaylistCache,
    required this.lbExplorationIndexMap,
  });

  @override
  State<ListenBrainzTab> createState() => _ListenBrainzTabState();
}

class _ListenBrainzTabState extends State<ListenBrainzTab> {
  final _exploration = _ExplorationData();
  final _year = _SectionData();
  final _top = _SectionData();

  @override
  void initState() {
    super.initState();
    _fetchExploration();
    _fetchYear();
    _fetchTop();
  }

  Future<void> _fetchExploration({int? index}) async {
    if (_exploration.fetching) return;
    _exploration.fetching = true;
    final targetIndex = index ?? _exploration.index;

    final cachedId = widget.lbExplorationIndexMap[targetIndex];
    if (cachedId != null) {
      final cached = widget.lbPlaylistCache[cachedId];
      if (cached != null) {
        if (!mounted) return;
        setState(() {
          _exploration.recordings = cached.recordings;
          _exploration.playlistTitle = cached.title;
          _exploration.playlistCount = cached.count;
          _exploration.loading = false;
          _exploration.index = targetIndex;
        });
        _exploration.fetching = false;
        return;
      }
    }

    setState(() => _exploration.loading = true);
    final result = await BridgeService.instance.getLBRecommendations(
      recType: 'weekly-exploration',
      count: 20,
      index: targetIndex,
    );
    if (!mounted) return;
    setState(() {
      _exploration.recordings = result.recordings;
      _exploration.playlistTitle = result.playlistTitle;
      _exploration.playlistCount = result.playlistCount ?? 0;
      if (result.playlistId != null) {
        widget.lbPlaylistCache[result.playlistId!] = (
          recordings: result.recordings,
          title: result.playlistTitle,
          count: result.playlistCount ?? 0,
        );
        widget.lbExplorationIndexMap[targetIndex] = result.playlistId!;
      }
      _exploration.loading = false;
      if (index != null) _exploration.index = targetIndex;
    });
    _exploration.fetching = false;
  }

  Future<void> _fetchYear() async {
    setState(() => _year.loading = true);
    final result = await BridgeService.instance.getLBRecommendations(
      recType: 'year',
    );
    if (!mounted) return;
    setState(() {
      _year.recordings = result.recordings;
      _year.loading = false;
    });
  }

  Future<void> _fetchTop() async {
    setState(() => _top.loading = true);
    final result = await BridgeService.instance.getLBRecommendations(
      recType: 'top',
      count: 20,
    );
    if (!mounted) return;
    setState(() {
      _top.recordings = result.recordings;
      _top.loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final scale = AppTheme.spaceScale(context);

    return SingleChildScrollView(
      padding: EdgeInsets.symmetric(vertical: AppTheme.spaceSM * scale),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          RecommendationSection(
            title: 'Weekly Exploration',
            icon: Icons.explore,
            recordings: _exploration.recordings,
            loading: _exploration.loading,
            subtitle: _exploration.playlistTitle,
            onPrevious: () => _fetchExploration(index: _exploration.index + 1),
            onNext: () => _fetchExploration(index: _exploration.index - 1),
            canGoNext: _exploration.index > 0,
            canGoPrevious: _exploration.index < _exploration.playlistCount - 1,
            navigating: _exploration.fetching,
          ),
          RecommendationSection(
            title: 'Year in Music',
            icon: Icons.auto_awesome,
            recordings: _year.recordings,
            loading: _year.loading,
          ),
          RecommendationSection(
            title: 'Top Recommendations',
            icon: Icons.trending_up,
            recordings: _top.recordings,
            loading: _top.loading,
          ),
        ],
      ),
    );
  }
}
