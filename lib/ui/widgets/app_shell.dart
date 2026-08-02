import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/home_page.dart';

class AppShell extends StatelessWidget {
  final Widget body;
  final String title;
  final PreferredSizeWidget? bottom;
  final Widget? floatingActionButton;

  const AppShell({
    super.key,
    required this.body,
    required this.title,
    this.bottom,
    this.floatingActionButton,
  });

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final isDesktop = AppTheme.isDesktop(context);

    return Scaffold(
      appBar: AppBar(
        leading: !isDesktop
            ? IconButton(
                icon: const Icon(Icons.menu),
                onPressed: () => isExpandedNotifier.value = true,
              )
            : null,
        title: Text(title, style: textTheme.titleLarge),
        bottom: bottom,
      ),
      body: body,
      floatingActionButton: floatingActionButton,
    );
  }
}
