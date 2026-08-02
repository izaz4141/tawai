import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class ListSwitch extends StatelessWidget {
  final String title;
  final String subtitle;
  final ValueNotifier<bool> valueListenable;
  final bool? defaultValue;
  final void Function(bool)? onChanged;
  final bool enabled;

  const ListSwitch({
    super.key,
    required this.title,
    required this.subtitle,
    required this.valueListenable,
    this.defaultValue,
    this.onChanged,
    this.enabled = true,
  });

  void _handleToggle() {
    if (!enabled) return;
    final newValue = !valueListenable.value;
    if (onChanged != null) {
      onChanged!(newValue);
    } else {
      valueListenable.value = newValue;
    }
  }

  void _reset() {
    if (defaultValue == null) return;
    if (onChanged != null) {
      onChanged!(defaultValue!);
    } else {
      valueListenable.value = defaultValue!;
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;
    final iconScale = AppTheme.iconScale(context);

    return ValueListenableBuilder<bool>(
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
              Switch(
                value: value,
                onChanged: enabled ? (_) => _handleToggle() : null,
              ),
              if (defaultValue != null && value != defaultValue)
                IconButton(
                  icon: const Icon(Icons.arrow_back),
                  iconSize: AppTheme.iconSM * iconScale,
                  onPressed: _reset,
                ),
            ],
          ),
          onTap: enabled ? _handleToggle : null,
        );
      },
    );
  }
}
