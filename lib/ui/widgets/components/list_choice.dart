import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class ListChoice<T> extends StatelessWidget {
  final String title;
  final String subtitle;
  final ValueListenable<T> valueListenable;
  final List<(T, String, IconData?)> items;
  final T? defaultValue;
  final void Function(T)? onChanged;
  final bool enabled;

  const ListChoice({
    super.key,
    required this.title,
    required this.subtitle,
    required this.valueListenable,
    required this.items,
    this.defaultValue,
    this.onChanged,
    this.enabled = true,
  });

  void _handleChange(T newValue) {
    if (!enabled) return;
    if (onChanged != null) {
      onChanged!(newValue);
    } else if (valueListenable is ValueNotifier<T>) {
      (valueListenable as ValueNotifier<T>).value = newValue;
    }
  }

  void _reset() {
    if (defaultValue == null || !enabled) return;
    if (onChanged != null) {
      onChanged!(defaultValue!);
    } else if (valueListenable is ValueNotifier<T>) {
      (valueListenable as ValueNotifier<T>).value = defaultValue!;
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final iconScale = AppTheme.iconScale(context);

    return ValueListenableBuilder<T>(
      valueListenable: valueListenable,
      builder: (context, value, _) {
        return ListTile(
          title: Text(title, style: textTheme.bodyMedium),
          subtitle: subtitle.isNotEmpty
              ? Text(
                  subtitle,
                  style: textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                )
              : null,
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              SegmentedButton<T>(
                segments: [
                  for (final item in items)
                    ButtonSegment<T>(
                      value: item.$1,
                      label: Text(item.$2),
                      icon: item.$3 != null ? Icon(item.$3) : null,
                    ),
                ],
                selected: {value},
                onSelectionChanged: enabled
                    ? (selection) => _handleChange(selection.first)
                    : null,
                showSelectedIcon: false,
                style: ButtonStyle(
                  foregroundColor: WidgetStateProperty.resolveWith((states) {
                    if (states.contains(WidgetState.selected)) {
                      return colors.onSecondaryContainer;
                    }
                    return colors.onSurfaceVariant;
                  }),
                  backgroundColor: WidgetStateProperty.resolveWith((states) {
                    if (states.contains(WidgetState.selected)) {
                      return colors.secondaryContainer;
                    }
                    if (states.contains(WidgetState.hovered)) {
                      return colors.surfaceContainerHighest;
                    }
                    return colors.surfaceContainer;
                  }),
                ),
              ),
              if (defaultValue != null && value != defaultValue)
                IconButton(
                  icon: const Icon(Icons.arrow_back),
                  iconSize: AppTheme.iconSM * iconScale,
                  onPressed: enabled ? _reset : null,
                ),
            ],
          ),
        );
      },
    );
  }
}
