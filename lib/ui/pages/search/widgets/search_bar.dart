import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/pages/search/search_types.dart';

class SearchInputBar extends StatelessWidget {
  final TextEditingController controller;
  final String hintText;
  final SearchMode mode;
  final bool isSearching;
  final bool showAutoDownload;
  final bool isAutoDownloading;
  final VoidCallback onSearch;
  final VoidCallback? onAutoDownload;
  final ValueChanged<String>? onSubmitted;

  const SearchInputBar({
    super.key,
    required this.controller,
    required this.hintText,
    this.mode = SearchMode.standard,
    this.isSearching = false,
    this.showAutoDownload = false,
    this.isAutoDownloading = false,
    required this.onSearch,
    this.onAutoDownload,
    this.onSubmitted,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppTheme.radiusSM),
        side: BorderSide(
          color: Theme.of(context).colorScheme.outlineVariant,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: AppTheme.spaceSM),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                decoration: InputDecoration(
                  hintText: hintText,
                  border: InputBorder.none,
                  isDense: true,
                  contentPadding: EdgeInsets.symmetric(
                    vertical: AppTheme.spaceSM,
                  ),
                ),
                onSubmitted: onSubmitted ?? (_) => onSearch(),
              ),
            ),
            if (isSearching)
              Padding(
                padding: const EdgeInsets.all(AppTheme.spaceXS),
                child: SizedBox(
                  width: AppTheme.iconSM,
                  height: AppTheme.iconSM,
                  child: const CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            else
              IconButton(
                icon: const Icon(Icons.search),
                tooltip: 'Search',
                onPressed: onSearch,
                visualDensity: VisualDensity.compact,
              ),
            if (showAutoDownload && onAutoDownload != null)
              isAutoDownloading
                  ? Padding(
                      padding: const EdgeInsets.all(AppTheme.spaceXS),
                      child: SizedBox(
                        width: AppTheme.iconSM,
                        height: AppTheme.iconSM,
                        child:
                            const CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : IconButton(
                      icon: const Icon(Icons.bolt),
                      tooltip: 'Auto-download best match',
                      onPressed: onAutoDownload,
                      visualDensity: VisualDensity.compact,
                    ),
          ],
        ),
      ),
    );
  }
}
