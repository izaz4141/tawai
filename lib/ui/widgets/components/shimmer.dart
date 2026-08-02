import 'package:flutter/material.dart';

class ShimmerWidget extends StatelessWidget {
  final double height;
  final double? width;
  final double borderRadius;
  final AnimationController controller;

  const ShimmerWidget({
    super.key,
    required this.height,
    this.width,
    this.borderRadius = 4,
    required this.controller,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return AnimatedBuilder(
      animation: controller,
      builder: (_, __) => Container(
        height: height,
        width: width ?? double.infinity,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(borderRadius),
          gradient: LinearGradient(
            begin: Alignment(controller.value * 2 - 1, 0),
            end: Alignment(controller.value * 2 + 1, 0),
            colors: [
              colors.surfaceContainerHighest.withAlpha(40),
              colors.surfaceContainerHighest.withAlpha(140),
              colors.surfaceContainerHighest.withAlpha(40),
            ],
          ),
        ),
      ),
    );
  }
}
