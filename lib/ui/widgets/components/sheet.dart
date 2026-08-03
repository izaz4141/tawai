import 'package:flutter/material.dart';

class GesturePill extends StatelessWidget {
  const GesturePill({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Center(
        child: Container(
          width: 32,
          height: 4,
          decoration: BoxDecoration(
            color: Theme.of(
              context,
            ).colorScheme.onSurface.withValues(alpha: 0.3),
            borderRadius: BorderRadius.circular(2),
          ),
        ),
      ),
    );
  }
}

class SheetActionItem extends StatelessWidget {
  const SheetActionItem({
    super.key,
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(leading: Icon(icon), title: Text(label), onTap: onTap);
  }
}

class Sheet extends StatelessWidget {
  const Sheet({
    super.key,
    this.header,
    this.body,
    this.showDivider = true,
    this.draggable = false,
    this.initialChildSize = 0.6,
    this.minChildSize = 0.3,
    this.maxChildSize = 0.9,
    this.bodyBuilder,
  }) : assert(
         !draggable || bodyBuilder != null,
         'Must provide bodyBuilder when draggable',
       );

  final Widget? header;
  final Widget? body;
  final bool showDivider;
  final bool draggable;
  final double initialChildSize;
  final double minChildSize;
  final double maxChildSize;
  final Widget Function(ScrollController)? bodyBuilder;

  @override
  Widget build(BuildContext context) {
    if (draggable) {
      return ClipRRect(
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
        child: Container(
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: DraggableScrollableSheet(
            initialChildSize: initialChildSize,
            minChildSize: minChildSize,
            maxChildSize: maxChildSize,
            expand: false,
            builder: (context, scrollCtrl) {
              return SafeArea(
                top: false,
                child: Column(
                  mainAxisSize: MainAxisSize.max,
                  children: [
                    const GesturePill(),
                    ?header,
                    if (showDivider) const Divider(height: 1),
                    Expanded(child: bodyBuilder!(scrollCtrl)),
                  ],
                ),
              );
            },
          ),
        ),
      );
    }

    return SafeArea(
      top: false,
      child: Column(
        mainAxisSize: MainAxisSize.max,
        children: [
          const GesturePill(),
          ?header,
          if (showDivider && header != null) const Divider(height: 1),
          if (body != null)
            Expanded(
              child: ScrollConfiguration(
                behavior: ScrollConfiguration.of(
                  context,
                ).copyWith(scrollbars: false),
                child: SingleChildScrollView(child: body!),
              ),
            ),
        ],
      ),
    );
  }
}
