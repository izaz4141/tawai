import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/pages/settings/tabs/global_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/system_tab.dart';
import 'package:tawai/ui/pages/settings/tabs/discovery_tab.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  bool _isAdmin = false;

  @override
  void initState() {
    super.initState();
    _isAdmin = SettingsManager.currentUser.value?.role == 'admin';
    _tabController = TabController(length: _isAdmin ? 3 : 2, vsync: this);
    SettingsManager.currentUser.addListener(_onUserChanged);
  }

  void _onUserChanged() {
    final isAdmin = SettingsManager.currentUser.value?.role == 'admin';
    if (isAdmin != _isAdmin) {
      _tabController.dispose();
      _isAdmin = isAdmin;
      _tabController = TabController(length: _isAdmin ? 3 : 2, vsync: this);
      setState(() {});
    }
  }

  @override
  void dispose() {
    SettingsManager.currentUser.removeListener(_onUserChanged);
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    final tabs = <Tab>[];
    final tabViews = <Widget>[];

    if (_isAdmin) {
      tabs.add(const Tab(text: 'Global'));
      tabViews.add(const SettingsGlobalTab());
    }
    tabs.addAll([const Tab(text: 'System'), const Tab(text: 'Discovery')]);
    tabViews.addAll([const SettingsSystemTab(), const SettingsDiscoveryTab()]);

    final tabBar = TabBar(
      isScrollable: true,
      controller: _tabController,
      labelStyle: textTheme.bodyMedium?.copyWith(color: colors.primary),
      unselectedLabelStyle: textTheme.bodyMedium,
      splashBorderRadius: BorderRadius.vertical(
        top: Radius.circular(AppTheme.radiusLG),
      ),
      tabs: tabs,
    );

    return AppShell(
      title: 'Settings',
      bottom: tabBar,
      body: Column(
        children: [
          Expanded(
            child: TabBarView(controller: _tabController, children: tabViews),
          ),
        ],
      ),
    );
  }
}
