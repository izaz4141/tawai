import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class AlbumTrackRow extends StatelessWidget {
  final int? position;
  final String title;
  final String? subtitle;
  final Color? subtitleColor;
  final IconData icon;
  final Color iconColor;
  final FontWeight? titleWeight;
  final Color? titleColor;
  final bool warningIcon;
  final Widget? trailing;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;
  final bool isSelected;

  const AlbumTrackRow({
    super.key,
    this.position,
    required this.title,
    this.subtitle,
    this.subtitleColor,
    required this.icon,
    required this.iconColor,
    this.titleWeight,
    this.titleColor,
    this.warningIcon = false,
    this.trailing,
    this.onTap,
    this.onLongPress,
    this.isSelected = false,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final scale = AppTheme.spaceScale(context);

    return Container(
      color: isSelected ? colors.primaryContainer.withAlpha(100) : null,
      child: InkWell(
        onTap: onTap,
        onLongPress: onLongPress,
        child: Padding(
          padding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceMD * scale,
            vertical: AppTheme.spaceSM,
          ),
          child: Row(
            children: [
              if (position != null)
                SizedBox(
                  width: 24,
                  child: Text(
                    '$position',
                    style: Theme.of(
                      context,
                    ).textTheme.bodySmall?.copyWith(color: colors.outline),
                  ),
                ),
              const SizedBox(width: AppTheme.spaceSM),
              Icon(icon, size: AppTheme.iconSM, color: iconColor),
              const SizedBox(width: AppTheme.spaceSM),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodyMedium
                                ?.copyWith(
                                  fontWeight: titleWeight,
                                  color: titleColor,
                                ),
                          ),
                        ),
                        if (warningIcon)
                          const Padding(
                            padding: EdgeInsets.only(left: 4),
                            child: Icon(
                              Icons.warning_amber_rounded,
                              size: 14,
                              color: Colors.orange,
                            ),
                          ),
                      ],
                    ),
                    if (subtitle != null)
                      Text(
                        subtitle!,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: subtitleColor ?? colors.outline,
                        ),
                      ),
                  ],
                ),
              ),
              ?trailing,
            ],
          ),
        ),
      ),
    );
  }
}
