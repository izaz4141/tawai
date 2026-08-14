import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/library/filterable_list.dart';
import 'package:tawai/ui/pages/library/modal/library_filter_dialog.dart';

class LibrarySearchFilter extends StatefulWidget {
  final int tabIndex;
  final LibraryFilters filters;
  final LibraryFilterOptions options;
  final bool showSearch;
  final bool showFilter;
  final double rightPadding;
  final ValueChanged<String> onQueryChanged;
  final ValueChanged<LibraryFilters> onFiltersChanged;

  const LibrarySearchFilter({
    super.key,
    required this.tabIndex,
    required this.filters,
    required this.options,
    this.showSearch = true,
    this.showFilter = true,
    this.rightPadding = 0,
    required this.onQueryChanged,
    required this.onFiltersChanged,
  });

  @override
  State<LibrarySearchFilter> createState() => _LibrarySearchFilterState();
}

class _LibrarySearchFilterState extends State<LibrarySearchFilter> {
  late TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.filters.query);
  }

  @override
  void didUpdateWidget(LibrarySearchFilter old) {
    super.didUpdateWidget(old);
    if (widget.filters.query != old.filters.query &&
        widget.filters.query != _controller.text) {
      _controller.text = widget.filters.query;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  String get _hint {
    switch (widget.tabIndex) {
      case 0:
        return 'Search tracks…';
      case 1:
        return 'Search albums…';
      case 2:
        return 'Search artists…';
      case 3:
        return 'Search playlists…';
      default:
        return 'Search…';
    }
  }

  void _openFilterDialog() {
    LibraryFilterDialog.show(
      context,
      filters: widget.filters,
      options: widget.options,
      tabIndex: widget.tabIndex,
    ).then((result) {
      if (result != null && mounted) {
        widget.onFiltersChanged(result);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final iconScale = AppTheme.iconScale(context);
    final active = widget.filters.hasActiveFilters;

    if (!widget.showSearch) {
      return const SizedBox.shrink();
    }

    return Padding(
      padding: EdgeInsets.fromLTRB(
        AppTheme.spaceMD,
        AppTheme.spaceXS,
        AppTheme.spaceMD + widget.rightPadding,
        0,
      ),
      child: TextField(
        controller: _controller,
        onChanged: widget.onQueryChanged,
        style: Theme.of(context).textTheme.bodySmall,
        decoration: InputDecoration(
          hintText: _hint,
          prefixIcon: Padding(
            padding: EdgeInsets.only(
              left: AppTheme.spaceSM,
              right: AppTheme.spaceXS,
            ),
            child: Icon(
              Icons.search,
              size: AppTheme.iconSM * iconScale,
            ),
          ),
          prefixIconConstraints: const BoxConstraints(minWidth: 0, minHeight: 0),
          suffixIcon: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.filters.query.isNotEmpty)
                IconButton(
                  icon: Icon(
                    Icons.clear,
                    size: AppTheme.iconXS * iconScale,
                  ),
                  visualDensity: VisualDensity.compact,
                  onPressed: () {
                    _controller.clear();
                    widget.onQueryChanged('');
                  },
                ),
              if (widget.showFilter)
                IconButton(
                  icon: Badge.count(
                    count: widget.filters.activeCount,
                    isLabelVisible: active,
                    backgroundColor: colors.primary,
                    textStyle: TextStyle(
                      fontSize: AppTheme.textXS * iconScale,
                      color: colors.onPrimary,
                    ),
                    child: Icon(
                      Icons.filter_list,
                      size: AppTheme.iconSM * iconScale,
                      color: active ? colors.primary : null,
                    ),
                  ),
                  visualDensity: VisualDensity.compact,
                  onPressed: _openFilterDialog,
                ),
            ],
          ),
          isDense: true,
          contentPadding: EdgeInsets.symmetric(
            vertical: AppTheme.spaceSM * 0.5,
          ),
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(AppTheme.radiusSM),
            borderSide: BorderSide(color: colors.outline),
          ),
          filled: true,
          fillColor: colors.surfaceContainerHighest.withValues(alpha: 0.4),
        ),
      ),
    );
  }
}