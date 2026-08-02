import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import 'package:tawai/utils/platform_service.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/window_controls.dart';
import 'package:tawai/ui/widgets/app_drawer.dart';
import 'package:tawai/ui/widgets/mini_player.dart';
import 'package:tawai/ui/pages/system/system_page.dart';
import 'package:tawai/ui/pages/settings/settings_page.dart';
import 'package:tawai/ui/pages/library/library_page.dart';
import 'package:tawai/ui/pages/search/search_page.dart';
import 'package:tawai/ui/pages/identify/identify_page.dart';
import 'package:tawai/ui/pages/tools/tools_page.dart';
import 'package:tawai/ui/pages/discovery/discovery_page.dart';

/// Shared state for navigation index
final ValueNotifier<int> navIndexNotifier = ValueNotifier<int>(1);

/// Whether the mini nav is expanded
final ValueNotifier<bool> isExpandedNotifier = ValueNotifier<bool>(false);

const _playerPages = {1, 2, 3};

class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isDesktop = AppTheme.isDesktop(context);

        return Stack(
          children: [
            Column(
              children: [
                Expanded(
                  child: Scaffold(
                    body: Row(
                      children: [
                        if (isDesktop) const NavigationRailSection(),
                        const Expanded(child: _PageContent()),
                      ],
                    ),
                  ),
                ),
                ValueListenableBuilder<int>(
                  valueListenable: navIndexNotifier,
                  builder: (context, selectedIndex, _) {
                    final showPlayer = _playerPages.contains(selectedIndex);
                    if (!showPlayer) return const SizedBox.shrink();
                    final colors = Theme.of(context).colorScheme;
                    return Container(
                      color: colors.surface,
                      child: const MiniPlayer(),
                    );
                  },
                ),
              ],
            ),
            // Always include interactive sidebar so it can respond to swipes and hamburger menu
            const InteractiveSidebar(),
            if (!kIsWeb && PlatformService.isDesktop) ...[
              Positioned(
                top: 0,
                left: 0,
                right: 0,
                height: kToolbarHeight,
                child: GestureDetector(
                  behavior: HitTestBehavior.translucent,
                  onPanStart: (details) {
                    PlatformService().startDragging();
                  },
                  onDoubleTap: () async {
                    if (await PlatformService().isMaximized()) {
                      await PlatformService().unmaximize();
                    } else {
                      await PlatformService().maximize();
                    }
                  },
                ),
              ),
              Positioned(
                top: 0,
                right: 0,
                child: Material(
                  type: MaterialType.transparency,
                  child: WindowControls(),
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}

/// Right-side content that switches based on selected nav index
class _PageContent extends StatefulWidget {
  const _PageContent();

  @override
  State<_PageContent> createState() => _PageContentState();
}

class _PageContentState extends State<_PageContent> {
  final _navKeys = <int, GlobalKey<NavigatorState>>{
    1: GlobalKey<NavigatorState>(),
    2: GlobalKey<NavigatorState>(),
    3: GlobalKey<NavigatorState>(),
  };

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<int>(
      valueListenable: navIndexNotifier,
      builder: (context, index, _) {
        final page = switch (index) {
          1 => const LibraryPage(),
          2 => const SearchPage(),
          3 => const DiscoveryPage(),
          4 => const IdentifyPage(),
          5 => const ToolsPage(),
          6 => const SettingsPage(),
          7 => const SystemPage(),
          _ => const LibraryPage(),
        };
        if (_navKeys.containsKey(index)) {
          return Navigator(
            key: _navKeys[index],
            onGenerateRoute: (_) => MaterialPageRoute(builder: (_) => page),
          );
        }
        return page;
      },
    );
  }
}
