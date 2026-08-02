import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/app_shell.dart';
import 'package:tawai/ui/widgets/dialog/tag_editor.dart';
import 'package:tawai/ui/pages/tools/renamer_page.dart';
import 'package:tawai/ui/pages/tools/duplicate_finder_page.dart';
import 'package:tawai/ui/pages/tools/missing_metadata_page.dart';
import 'package:tawai/ui/pages/tools/lyrics_manager_page.dart';
import 'package:tawai/ui/pages/tools/stats_page.dart';

class _ToolEntry {
  final IconData icon;
  final String label;
  final String description;
  final VoidCallback onTap;
  const _ToolEntry({
    required this.icon,
    required this.label,
    required this.description,
    required this.onTap,
  });
}

class ToolsPage extends StatelessWidget {
  const ToolsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final isDesktop = AppTheme.isDesktop(context);

    final tools = [
      _ToolEntry(
        icon: Icons.drive_file_rename_outline,
        label: 'File Renamer',
        description: 'Batch rename and organize files by tag pattern',
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const RenamerPage()),
        ),
      ),
      _ToolEntry(
        icon: Icons.copy_all,
        label: 'Duplicate Finder',
        description: 'Find duplicate tracks by fingerprint or metadata',
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const DuplicateFinderPage()),
        ),
      ),
      _ToolEntry(
        icon: Icons.fact_check_outlined,
        label: 'Missing Metadata',
        description: 'Find tracks missing essential tags',
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const MissingMetadataPage()),
        ),
      ),
      _ToolEntry(
        icon: Icons.lyrics,
        label: 'Lyrics Manager',
        description: 'Search, edit, and attach lyrics to tracks',
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const LyricsManagerPage()),
        ),
      ),
      _ToolEntry(
        icon: Icons.bar_chart,
        label: 'Collection Statistics',
        description: 'Library stats, format distribution, naming compliance',
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute(builder: (_) => const StatsPage()),
        ),
      ),
      _ToolEntry(
        icon: Icons.edit_note,
        label: 'Tag Editor',
        description: 'Quickly edit tags on any audio file',
        onTap: () => showTagEditorDialog(context),
      ),
    ];

    return AppShell(
      title: 'Tools',
      body: isDesktop
          ? GridView.extent(
              padding: EdgeInsets.all(AppTheme.spaceLG * AppTheme.spaceScale(context)),
              maxCrossAxisExtent: 260,
              crossAxisSpacing: AppTheme.spaceMD,
              mainAxisSpacing: AppTheme.spaceMD,
              childAspectRatio: 1.1,
              children: tools.map((t) => _buildCard(t, colors, textTheme)).toList(),
            )
          : ListView.separated(
              padding: EdgeInsets.all(AppTheme.spaceMD * AppTheme.spaceScale(context)),
              itemCount: tools.length,
              separatorBuilder: (_, _) => SizedBox(height: AppTheme.spaceSM),
              itemBuilder: (context, index) => _buildListTile(tools[index], colors, textTheme),
            ),
    );
  }

  Widget _buildCard(_ToolEntry tool, ColorScheme colors, TextTheme textTheme) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: tool.onTap,
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.spaceLG),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(tool.icon, size: 48, color: colors.primary),
              SizedBox(height: AppTheme.spaceSM),
              Text(tool.label, style: textTheme.titleMedium, textAlign: TextAlign.center),
              SizedBox(height: AppTheme.spaceXS),
              Text(
                tool.description,
                style: textTheme.bodySmall?.copyWith(color: colors.onSurfaceVariant),
                textAlign: TextAlign.center,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildListTile(_ToolEntry tool, ColorScheme colors, TextTheme textTheme) {
    return Card(
      child: ListTile(
        leading: Icon(tool.icon, color: colors.primary, size: 32),
        title: Text(tool.label),
        subtitle: Text(tool.description, style: textTheme.bodySmall),
        trailing: const Icon(Icons.chevron_right),
        onTap: tool.onTap,
      ),
    );
  }
}
