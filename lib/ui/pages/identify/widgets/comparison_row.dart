import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class ComparisonEditableRow extends StatelessWidget {
  final String label;
  final TextEditingController controller;
  final String remoteValue;
  final String? originalValue;
  final TextTheme textTheme;
  final ColorScheme colors;

  const ComparisonEditableRow({
    super.key,
    required this.label,
    required this.controller,
    required this.remoteValue,
    this.originalValue,
    required this.textTheme,
    required this.colors,
  });

  @override
  Widget build(BuildContext context) {
    final orig = originalValue ?? '';
    return Padding(
      padding: EdgeInsets.symmetric(
        vertical: AppTheme.spaceXS * AppTheme.spaceScale(context),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: AppTheme.spaceXXL * 2 * AppTheme.spaceScale(context),
            child: Text(
              label,
              style: textTheme.labelSmall?.copyWith(
                fontWeight: FontWeight.w600,
                color: colors.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: TextField(
              controller: controller,
              style: textTheme.bodySmall,
              decoration: InputDecoration(
                isDense: true,
                contentPadding: EdgeInsets.symmetric(
                  horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
                  vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
                ),
                border: const OutlineInputBorder(),
              ),
            ),
          ),
          SizedBox(
            width: AppTheme.spaceLG * 2 * AppTheme.spaceScale(context),
            child: ListenableBuilder(
              listenable: controller,
              builder: (context, _) {
                final localValue = controller.text;
                final diff = localValue != remoteValue;
                final applied = !diff && localValue != orig;

                IconData icon;
                Color iconColor;
                VoidCallback? onPress;
                String tooltip;

                if (diff) {
                  icon = Icons.arrow_back;
                  iconColor = colors.primary;
                  onPress = () => controller.text = remoteValue;
                  tooltip = 'Use remote value';
                } else if (applied) {
                  icon = Icons.undo;
                  iconColor = colors.tertiary;
                  onPress = () => controller.text = orig;
                  tooltip = 'Revert to original';
                } else {
                  icon = Icons.arrow_back;
                  iconColor = colors.outline.withAlpha(80);
                  onPress = null;
                  tooltip = '';
                }

                return IconButton(
                  onPressed: onPress,
                  icon: Icon(
                    icon,
                    size: AppTheme.iconSM * AppTheme.iconScale(context),
                    color: iconColor,
                  ),
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                  tooltip: tooltip,
                );
              },
            ),
          ),
          Expanded(
            child: Text(
              remoteValue,
              style: textTheme.bodySmall,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

class ComparisonReadonlyRow extends StatelessWidget {
  final String label;
  final String current;
  final String remote;
  final bool applied;
  final VoidCallback? onApply;
  final VoidCallback? onRevert;
  final TextTheme textTheme;
  final ColorScheme colors;

  const ComparisonReadonlyRow({
    super.key,
    required this.label,
    required this.current,
    required this.remote,
    this.applied = false,
    this.onApply,
    this.onRevert,
    required this.textTheme,
    required this.colors,
  });

  @override
  Widget build(BuildContext context) {
    final hasApply = !applied && current != remote && onApply != null;
    final hasRevert = applied && onRevert != null;

    return Padding(
      padding: EdgeInsets.symmetric(
        vertical: AppTheme.spaceXS * AppTheme.spaceScale(context),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: AppTheme.spaceXXL * 2 * AppTheme.spaceScale(context),
            child: Text(
              label,
              style: textTheme.labelSmall?.copyWith(
                fontWeight: FontWeight.w600,
                color: colors.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Padding(
              padding: EdgeInsets.symmetric(
                vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
              ),
              child: Text(
                current,
                style: textTheme.bodySmall,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
          SizedBox(
            width: AppTheme.spaceLG * 2 * AppTheme.spaceScale(context),
            child: IconButton(
              onPressed: hasApply
                  ? onApply
                  : hasRevert
                  ? onRevert
                  : null,
              icon: Icon(
                hasApply
                    ? Icons.arrow_back
                    : hasRevert
                    ? Icons.undo
                    : Icons.arrow_back,
                size: AppTheme.iconSM * AppTheme.iconScale(context),
                color: hasApply
                    ? colors.primary
                    : hasRevert
                    ? colors.tertiary
                    : colors.outline.withAlpha(80),
              ),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(),
              tooltip: hasApply
                  ? 'Use remote value'
                  : hasRevert
                  ? 'Revert'
                  : '',
            ),
          ),
          Expanded(
            child: Padding(
              padding: EdgeInsets.symmetric(
                vertical: AppTheme.spaceSM * AppTheme.spaceScale(context),
              ),
              child: Text(
                remote,
                style: textTheme.bodySmall,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
