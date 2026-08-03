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
    final spaceScale = AppTheme.spaceScale(context);

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
          ? GridView.builder(
              padding: EdgeInsets.all(AppTheme.spaceLG * spaceScale),
              gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
                maxCrossAxisExtent: 340,
                crossAxisSpacing: AppTheme.spaceMD * spaceScale,
                mainAxisSpacing: AppTheme.spaceMD * spaceScale,
                mainAxisExtent: 220 * AppTheme.heightScale(context),
              ),
              itemCount: tools.length,
              itemBuilder: (context, index) =>
                  _buildCard(context, tools[index], colors, textTheme),
            )
          : ListView.separated(
              padding: EdgeInsets.all(
                AppTheme.spaceMD * AppTheme.spaceScale(context),
              ),
              itemCount: tools.length,
              separatorBuilder: (_, _) => SizedBox(height: AppTheme.spaceSM),
              itemBuilder: (context, index) =>
                  _buildListTile(context, tools[index], colors, textTheme),
            ),
    );
  }

  Widget _buildCard(
    BuildContext context,
    _ToolEntry tool,
    ColorScheme colors,
    TextTheme textTheme,
  ) {
    final scale = AppTheme.spaceScale(context);
    final titleHeight = (textTheme.titleMedium?.fontSize ?? 20) * 1.3;
    final descHeight = (textTheme.bodySmall?.fontSize ?? 12) * 1.3 * 2;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: tool.onTap,
        child: Padding(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceMD * scale,
            vertical: AppTheme.spaceLG * scale,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                tool.icon,
                size: AppTheme.iconXL * AppTheme.iconScale(context),
                color: colors.primary,
              ),
              SizedBox(height: AppTheme.spaceSM * scale),
              SizedBox(
                height: titleHeight,
                child: Text(
                  tool.label,
                  style: textTheme.titleMedium?.copyWith(height: 1.3),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              SizedBox(height: AppTheme.spaceXS * scale),
              SizedBox(
                height: descHeight,
                child: Text(
                  tool.description,
                  style: textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                    height: 1.3,
                  ),
                  textAlign: TextAlign.center,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildListTile(
    BuildContext context,
    _ToolEntry tool,
    ColorScheme colors,
    TextTheme textTheme,
  ) {
    return Card(
      child: ListTile(
        leading: Icon(
          tool.icon,
          color: colors.primary,
          size: AppTheme.iconLG * AppTheme.iconScale(context),
        ),
        title: Text(tool.label),
        subtitle: Text(tool.description, style: textTheme.bodySmall),
        trailing: const Icon(Icons.chevron_right),
        onTap: tool.onTap,
      ),
    );
  }
}
