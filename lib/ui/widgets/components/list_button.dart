import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

class ListButton extends StatelessWidget {
  final String title;
  final String subtitle;
  final VoidCallback? onPressed;
  final Widget? leading;
  final Widget trailing;
  final bool enabled;

  const ListButton({
    super.key,
    required this.title,
    required this.subtitle,
    this.onPressed,
    this.leading,
    required this.trailing,
    this.enabled = true,
  });

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
          child: AbsorbPointer(absorbing: !enabled, child: trailing),
        ),
      ),
    );
  }
}
