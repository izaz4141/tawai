import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

enum ListButtonType { icon, text, iconText }

class ListButton extends StatelessWidget {
  final String title;
  final String subtitle;
  final VoidCallback? onPressed;
  final Widget? leading;
  final ListButtonType type;
  final IconData? icon;
  final String? label;
  final bool enabled;

  const ListButton({
    super.key,
    required this.title,
    required this.subtitle,
    this.onPressed,
    this.leading,
    this.type = ListButtonType.iconText,
    this.icon,
    this.label,
    this.enabled = true,
  });

  Widget _buildButton(BuildContext context) {
    final onPressed = enabled ? this.onPressed : null;
    switch (type) {
      case ListButtonType.icon:
        return IconButton.filledTonal(
          onPressed: onPressed,
          icon: Icon(icon, size: AppTheme.iconMD * AppTheme.iconScale(context)),
        );
      case ListButtonType.text:
        return OutlinedButton(
          onPressed: onPressed,
          child: Text(label ?? 'Configure'),
        );
      case ListButtonType.iconText:
        return OutlinedButton.icon(
          onPressed: onPressed,
          icon: Icon(icon, size: AppTheme.iconMD * AppTheme.iconScale(context)),
          label: Text(label ?? 'Configure'),
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final colors = Theme.of(context).colorScheme;

    return ListTile(
      leading: leading != null
          ? IconTheme(
              data: IconThemeData(
                size: AppTheme.iconMD * AppTheme.iconScale(context),
                color: enabled ? colors.primary : colors.onSurfaceVariant,
              ),
              child: leading!,
            )
          : null,
      title: Text(
        title,
        style: textTheme.bodyMedium?.copyWith(
          color: enabled ? null : colors.onSurfaceVariant,
        ),
      ),
      subtitle: Text(
        subtitle,
        style: textTheme.bodySmall?.copyWith(
          color: enabled ? null : colors.onSurfaceVariant,
        ),
      ),
      enabled: enabled,
      trailing: SizedBox(
        width: 250 * AppTheme.spaceScale(context),
        child: Opacity(
          opacity: enabled ? 1.0 : 0.4,
          child: AbsorbPointer(
            absorbing: !enabled,
            child: _buildButton(context),
          ),
        ),
      ),
    );
  }
}
