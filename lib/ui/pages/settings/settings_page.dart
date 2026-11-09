import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/pages/settings/tabs/appearance_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/playback_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/library_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/downloads_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/connections_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/system_tab.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController = TabController(
    length: 6,
    vsync: this,
  );

  static const List<Tab> _tabs = [
    Tab(text: 'Appearance'),
    Tab(text: 'Playback'),
    Tab(text: 'Library'),
    Tab(text: 'Downloads'),
    Tab(text: 'Connections'),
    Tab(text: 'System'),
  ];

  static const List<Widget> _tabViews = [
    SettingsAppearanceTab(),
    SettingsPlaybackTab(),
    SettingsLibraryTab(),
    SettingsDownloadsTab(),
    SettingsConnectionsTab(),
    SettingsSystemTab(),
  ];

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
      isScrollable: true,
      controller: _tabController,
      labelStyle: textTheme.bodyMedium?.copyWith(color: colors.primary),
      unselectedLabelStyle: textTheme.bodyMedium,
      splashBorderRadius: BorderRadius.vertical(
        top: Radius.circular(AppTheme.radiusLG),
      ),
      tabs: _tabs,
    );

    return AppShell(
      title: 'Settings',
      bottom: tabBar,
      body: ValueListenableBuilder(
        valueListenable: SettingsManager.currentUser,
        // The tabs are fixed; only their contents depend on the role, so the
        // controller is never rebuilt when the signed-in user changes.
        builder: (context, _, _) => Column(
          children: [
            Expanded(
              child: TabBarView(
                controller: _tabController,
                children: _tabViews,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
