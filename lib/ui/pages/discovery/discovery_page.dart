import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/pages/discovery/tabs/listenbrainz_tab.dart';

class DiscoveryPage extends StatefulWidget {
  const DiscoveryPage({super.key});

  @override
  State<DiscoveryPage> createState() => _DiscoveryPageState();
}

class _DiscoveryPageState extends State<DiscoveryPage>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  final _lbPlaylistCache = <String, ({List<DiscoveryRecording> recordings, String? title, int count})>{};
  final _lbExplorationIndexMap = <int, String>{};

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 1, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    final tabBar = TabBar(
      controller: _tabController,
      isScrollable: true,
      labelStyle: textTheme.bodyMedium?.copyWith(color: colors.primary),
      unselectedLabelStyle: textTheme.bodyMedium,
      splashBorderRadius: BorderRadius.vertical(
        top: Radius.circular(AppTheme.radiusLG),
      ),
      tabs: const [Tab(text: 'ListenBrainz')],
    );

    return AppShell(
      title: 'Discovery',
      bottom: tabBar,
      body: TabBarView(
        controller: _tabController,
        children: [
          ListenBrainzTab(
            lbPlaylistCache: _lbPlaylistCache,
            lbExplorationIndexMap: _lbExplorationIndexMap,
          ),
        ],
      ),
    );
  }
}
