import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/library/modal/library_filter_dialog.dart';

class LibrarySearchFilter extends StatefulWidget {
  final int tabIndex;
  final String query;
  final String? selectedSource;
  final List<String> availableSources;
  final bool showSearch;
  final double rightPadding;
  final ValueChanged<String> onQueryChanged;
  final ValueChanged<String?> onSourceChanged;

  const LibrarySearchFilter({
    super.key,
    required this.tabIndex,
    required this.query,
    required this.selectedSource,
    required this.availableSources,
    this.showSearch = true,
    this.rightPadding = 0,
    required this.onQueryChanged,
    required this.onSourceChanged,
  });

  @override
  State<LibrarySearchFilter> createState() => _LibrarySearchFilterState();
}

class _LibrarySearchFilterState extends State<LibrarySearchFilter> {
  late TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.query);
  }

  @override
  void didUpdateWidget(LibrarySearchFilter old) {
    super.didUpdateWidget(old);
    if (widget.query != old.query && widget.query != _controller.text) {
      _controller.text = widget.query;
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
      selectedSource: widget.selectedSource,
      availableSources: widget.availableSources,
    ).then((source) {
      if (source != widget.selectedSource && mounted) {
        widget.onSourceChanged(source);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    if (!widget.showSearch) {
      return const SizedBox.shrink();
    }

    return Padding(
      padding: EdgeInsets.fromLTRB(16, 8, 16 + widget.rightPadding, 4),
      child: TextField(
        controller: _controller,
        onChanged: widget.onQueryChanged,
        decoration: InputDecoration(
          hintText: _hint,
          prefixIcon: const Icon(Icons.search, size: 20),
          suffixIcon: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.query.isNotEmpty)
                IconButton(
                  icon: const Icon(Icons.clear, size: 18),
                  onPressed: () {
                    _controller.clear();
                    widget.onQueryChanged('');
                  },
                ),
              IconButton(
                icon: Icon(
                  widget.selectedSource != null
                      ? Icons.filter_list
                      : Icons.filter_list,
                  size: 20,
                  color: widget.selectedSource != null ? colors.primary : null,
                ),
                onPressed: _openFilterDialog,
              ),
            ],
          ),
          isDense: true,
          contentPadding: const EdgeInsets.symmetric(vertical: 10),
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(AppTheme.radiusLG),
            borderSide: BorderSide(color: colors.outline),
          ),
          filled: true,
          fillColor: colors.surfaceContainerHighest.withValues(alpha: 0.4),
        ),
      ),
    );
  }
}
