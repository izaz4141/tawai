import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/pages/system/tabs/app_view.dart';
import 'package:tawai/ui/pages/system/tabs/deps_view.dart';
import 'package:tawai/ui/pages/system/tabs/info_view.dart';

class SystemPage extends StatelessWidget {
  const SystemPage({super.key});

  @override
  Widget build(BuildContext context) {
    return AppShell(
      title: 'System',
      body: ListView(
        children: [
          const SystemApp(),
          SizedBox(height: AppTheme.spaceXL),
          const Divider(),
          const SystemInfo(),
          const Divider(),
          const SystemDeps(),
          SizedBox(height: 120 * AppTheme.heightScale(context)),
        ],
      ),
    );
  }
}
