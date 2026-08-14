import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/library/filterable_list.dart';

class LibraryFilterDialog extends StatefulWidget {
  final LibraryFilters filters;
  final LibraryFilterOptions options;
  final int tabIndex;

  const LibraryFilterDialog({
    super.key,
    required this.filters,
    required this.options,
    required this.tabIndex,
  });

  static Future<LibraryFilters?> show(
    BuildContext context, {
    required LibraryFilters filters,
    required LibraryFilterOptions options,
    required int tabIndex,
  }) {
    return showDialog<LibraryFilters>(
      context: context,
      builder: (_) => LibraryFilterDialog(
        filters: filters,
        options: options,
        tabIndex: tabIndex,
      ),
    );
  }

  @override
  State<LibraryFilterDialog> createState() => _LibraryFilterDialogState();
}

class _LibraryFilterDialogState extends State<LibraryFilterDialog> {
  late LibraryFilters _draft;

  @override
  void initState() {
    super.initState();
    _draft = widget.filters.copy();
  }

  void _toggle(Set<String> set, String value) {
    setState(() {
      if (!set.remove(value)) set.add(value);
    });
  }

  void _clearAll() {
    setState(() {
      _draft = LibraryFilters(query: _draft.query);
    });
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final iconScale = AppTheme.iconScale(context);

    return AlertDialog(
      title: const Text('Filter'),
      contentPadding: EdgeInsets.fromLTRB(
        AppTheme.spaceLG,
        AppTheme.spaceMD,
        AppTheme.spaceLG,
        0,
      ),
      content: SizedBox(
        width: AppTheme.dialogWidth(context),
        child: ListView(
          shrinkWrap: true,
          children: [
            if (widget.options.sources.isNotEmpty)
              _buildSection(
                'Sources',
                Icons.storage,
                widget.options.sources,
                _draft.sources,
              ),
            if (widget.options.genres.isNotEmpty)
              _buildSection(
                'Genres',
                Icons.music_note,
                widget.options.genres,
                _draft.genres,
              ),
            if (widget.options.years.isNotEmpty)
              _buildSection(
                'Years',
                Icons.calendar_today,
                widget.options.years,
                _draft.years,
              ),
            if (widget.options.sources.isEmpty &&
                widget.options.genres.isEmpty &&
                widget.options.years.isEmpty)
              Text(
                'No filters available for this tab.',
                style: textTheme.bodyMedium?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _draft.hasActiveFilters ? _clearAll : null,
          child: const Text('Clear'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_draft),
          child: const Text('Apply'),
        ),
      ],
    );
  }

  Widget _buildSection(
    String title,
    IconData icon,
    List<String> options,
    Set<String> selected,
  ) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final iconScale = AppTheme.iconScale(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: AppTheme.spaceSM),
          child: Row(
            children: [
              Icon(
                icon,
                size: AppTheme.iconSM * iconScale,
                color: colors.primary,
              ),
              SizedBox(width: AppTheme.spaceXS),
              Text(title, style: textTheme.titleSmall),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(top: AppTheme.spaceXS),
          child: Wrap(
            spacing: AppTheme.spaceXS,
            runSpacing: AppTheme.spaceXS,
            children: options.map((option) {
              return FilterChip(
                label: Text(option),
                selected: selected.contains(option),
                visualDensity: VisualDensity.compact,
                onSelected: (_) => _toggle(selected, option),
              );
            }).toList(),
          ),
        ),
      ],
    );
  }
}
