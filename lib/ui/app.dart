import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:dynamic_color/dynamic_color.dart';

import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/home_page.dart';
import 'package:tawai/ui/pages/login/login_page.dart';
import 'package:tawai/ui/widgets/dialog/permission_dialog.dart';
import 'package:tawai/utils/settings.dart';
import 'package:tawai/utils/platform_service.dart';

import 'package:rinf/rinf.dart';

// Global navigator key for accessing context from intent handlers
final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();

class Tawai extends StatefulWidget {
  const Tawai({super.key});

  @override
  State<Tawai> createState() => _TawaiState();
}

class _TawaiState extends State<Tawai> {
  /// This `AppLifecycleListener` is responsible for the
  /// graceful shutdown of the async runtime in Rust.
  /// If you don't care about
  /// properly dropping Rust objects before shutdown,
  /// creating this listener is not necessary.
  late final AppLifecycleListener _listener;

  @override
  void initState() {
    super.initState();
    _listener = AppLifecycleListener(
      onExitRequested: () async {
        if (!kIsWeb) {
          finalizeRust(); // This line shuts down the async Rust runtime.
        }
        return AppExitResponse.exit;
      },
    );

    if (PlatformService.isMobile) {
      if (SettingsManager.isFirstRun) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          final context = navigatorKey.currentContext;
          if (context != null) {
            showDialog(
              context: context,
              builder: (context) => const PermissionDialog(),
            );
          }
        });
      }
    }
  }

  @override
  void dispose() {
    _listener.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<ThemeMode>(
      valueListenable: SettingsManager.themeMode,
      builder: (context, themeMode, _) {
        return ValueListenableBuilder<bool>(
          valueListenable: SettingsManager.useDynamicColor,
          builder: (context, useDynamicColor, _) {
            return ValueListenableBuilder<int>(
              valueListenable: SettingsManager.customColor,
              builder: (context, customColorValue, _) {
                return DynamicColorBuilder(
                  builder: (ColorScheme? lightDynamic, ColorScheme? darkDynamic) {
                    final schemes = AppTheme.getColorSchemes(
                      lightDynamic,
                      darkDynamic,
                      customSeed: Color(customColorValue),
                      useDynamicColor: useDynamicColor,
                    );

                    return MaterialApp(
                      navigatorKey: navigatorKey,
                      title: 'Tawai',
                      scrollBehavior: AppTheme.dragBehavior,
                      theme: AppTheme.buildTheme(schemes.light, context),
                      darkTheme: AppTheme.buildTheme(schemes.dark, context),
                      themeMode: themeMode,
                      home: ValueListenableBuilder<bool>(
                        valueListenable: SettingsManager.requireLogin,
                        builder: (context, requireLogin, _) {
                          return ValueListenableBuilder<bool>(
                            valueListenable: SettingsManager.isLoggedIn,
                            builder: (context, isLoggedIn, _) {
                              if (requireLogin && !isLoggedIn) {
                                return LoginPage(
                                  onLoginSuccess: () {
                                    // Login handled by page, this callback just triggers rebuild
                                  },
                                );
                              }
                              return ValueListenableBuilder<String?>(
                                valueListenable: SettingsManager.currentUserId,
                                builder: (context, userId, _) =>
                                    HomePage(key: ValueKey(userId)),
                              );
                            },
                          );
                        },
                      ),
                      debugShowCheckedModeBanner: false,
                    );
                  },
                );
              },
            );
          },
        );
      },
    );
  }
}
