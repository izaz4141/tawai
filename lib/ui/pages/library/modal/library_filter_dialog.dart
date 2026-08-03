import 'package:flutter/material.dart';

class LibraryFilterDialog extends StatelessWidget {
  final String? selectedSource;
  final List<String> availableSources;

  const LibraryFilterDialog({
    super.key,
    required this.selectedSource,
    required this.availableSources,
  });

  static Future<String?> show(
    BuildContext context, {
    required String? selectedSource,
    required List<String> availableSources,
  }) {
    return showDialog<String>(
      context: context,
      builder: (_) => LibraryFilterDialog(
        selectedSource: selectedSource,
        availableSources: availableSources,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return AlertDialog(
      title: const Text('Filter by Source'),
      content: SizedBox(
        width: 320,
        child: ListView(
          shrinkWrap: true,
          children: [
            ListTile(
              dense: true,
              leading: Icon(
                selectedSource == null
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
                size: 20,
                color: selectedSource == null
                    ? colors.primary
                    : colors.onSurfaceVariant,
              ),
              title: Text(
                'All',
                style: textTheme.bodyMedium?.copyWith(
                  fontWeight: selectedSource == null
                      ? FontWeight.w600
                      : FontWeight.normal,
                ),
              ),
              onTap: () => Navigator.of(context).pop(null),
            ),
            if (availableSources.isNotEmpty) const Divider(height: 1),
            ...availableSources.map((source) {
              final isSelected = selectedSource == source;
              return ListTile(
                dense: true,
                leading: Icon(
                  isSelected
                      ? Icons.radio_button_checked
                      : Icons.radio_button_unchecked,
                  size: 20,
                  color: isSelected ? colors.primary : colors.onSurfaceVariant,
                ),
                title: Text(
                  source,
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: isSelected
                        ? FontWeight.w600
                        : FontWeight.normal,
                  ),
                ),
                onTap: () => Navigator.of(context).pop(source),
              );
            }),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
