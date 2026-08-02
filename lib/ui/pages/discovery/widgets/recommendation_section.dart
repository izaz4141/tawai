import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';
import 'package:tawai/ui/widgets/components/section_header.dart';
import 'package:tawai/ui/pages/discovery/widgets/recording_card.dart';
import 'package:tawai/src/bindings/bindings.dart';

class RecommendationSection extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<DiscoveryRecording>? recordings;
  final bool loading;
  final VoidCallback? onPrevious;
  final VoidCallback? onNext;
  final bool canGoNext;
  final bool canGoPrevious;
  final String? subtitle;
  final bool navigating;

  const RecommendationSection({
    super.key,
    required this.title,
    required this.icon,
    this.recordings,
    this.loading = false,
    this.onPrevious,
    this.onNext,
    this.canGoNext = false,
    this.canGoPrevious = false,
    this.subtitle,
    this.navigating = false,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final scale = AppTheme.spaceScale(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader(
          title: title,
          leading: Icon(icon),
          trailing: (onNext != null || onPrevious != null)
              ? Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (subtitle != null)
                      Padding(
                        padding: EdgeInsets.only(
                          right: AppTheme.spaceSM * scale,
                        ),
                        child: navigating
                            ? Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  SizedBox(
                                    width: 14,
                                    height: 14,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                      color: colors.onPrimaryContainer.withValues(alpha: 0.7),
                                    ),
                                  ),
                                  SizedBox(width: AppTheme.spaceSM * scale * 0.5),
                                  Text(
                                    subtitle!,
                                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                                      color: colors.onPrimaryContainer.withValues(alpha: 0.7),
                                    ),
                                  ),
                                ],
                              )
                            : Text(
                                subtitle!,
                                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                                  color: colors.onPrimaryContainer.withValues(alpha: 0.7),
                                ),
                              ),
                      ),
                    IconButton(
                      icon: const Icon(Icons.chevron_left),
                      onPressed: canGoPrevious && !navigating ? onPrevious : null,
                      visualDensity: VisualDensity.compact,
                      tooltip: 'Previous',
                    ),
                    IconButton(
                      icon: const Icon(Icons.chevron_right),
                      onPressed: canGoNext && !navigating ? onNext : null,
                      visualDensity: VisualDensity.compact,
                      tooltip: 'Next',
                    ),
                  ],
                )
              : null,
        ),
        SizedBox(height: AppTheme.spaceSM * scale),
        _buildContent(context),
        SizedBox(height: AppTheme.spaceLG * scale),
      ],
    );
  }

  Widget _buildContent(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    if (loading) {
      return Center(
        child: Padding(
          padding: EdgeInsets.all(AppTheme.spaceLG * AppTheme.spaceScale(context)),
          child: CircularProgressIndicator(color: colors.primary),
        ),
      );
    }

    if (recordings == null || recordings!.isEmpty) {
      return Padding(
        padding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceLG * AppTheme.spaceScale(context),
        ),
        child: Text(
          'No recommendations available yet.',
          style: textTheme.bodyMedium?.copyWith(
            color: colors.onSurfaceVariant,
          ),
        ),
      );
    }

    return SizedBox(
      height: 220,
      child: ListView.builder(
        scrollDirection: Axis.horizontal,
        padding: EdgeInsets.symmetric(
          horizontal: AppTheme.spaceSM * AppTheme.spaceScale(context),
        ),
        itemCount: recordings!.length,
        itemBuilder: (context, index) {
          final rec = recordings![index];
          return RecordingCard(
            recording: rec,
          );
        },
      ),
    );
  }
}
