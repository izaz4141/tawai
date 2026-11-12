import 'package:flutter/material.dart';

import 'package:tawai/ui/theme/app_theme.dart';

/// Standard centered empty state: an icon above a short message.
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.message,
    this.detail,
    this.iconColor,
    this.messageStyle,
    this.action,
    this.compact = false,
  });

  final IconData icon;
  final String message;
  final String? detail;
  final Color? iconColor;
  final TextStyle? messageStyle;
  final Widget? action;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final spaceScale = AppTheme.spaceScale(context);
    final iconScale = AppTheme.iconScale(context);
    final baseStyle = compact ? textTheme.bodyMedium : textTheme.bodyLarge;

    return Center(
      child: Padding(
        padding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceXL * spaceScale,
          vertical: AppTheme.spaceLG * spaceScale,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: (compact ? AppTheme.iconLG : AppTheme.iconXXL) * iconScale,
              color: iconColor ?? colors.outline,
            ),
            SizedBox(height: AppTheme.spaceMD * spaceScale),
            Text(
              message,
              textAlign: TextAlign.center,
              style: (messageStyle ?? baseStyle)?.copyWith(
                color: messageStyle?.color ?? colors.onSurfaceVariant,
              ),
            ),
            if (detail != null) ...[
              SizedBox(height: AppTheme.spaceXS * spaceScale),
              Text(
                detail!,
                textAlign: TextAlign.center,
                style: textTheme.bodySmall?.copyWith(color: colors.outline),
              ),
            ],
            if (action != null) ...[
              SizedBox(height: AppTheme.spaceLG * spaceScale),
              action!,
            ],
          ],
        ),
      ),
    );
  }
}

/// Scrollable wrapper for empty states that must fill and stay centered in a
/// pull-to-refreshable viewport.
class ScrollableEmptyState extends StatelessWidget {
  const ScrollableEmptyState({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: constraints.maxHeight),
            child: child,
          ),
        );
      },
    );
  }
}
