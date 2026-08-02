import 'package:flutter/material.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class ColorPickerDialog extends StatefulWidget {
  final Color initial;
  const ColorPickerDialog({super.key, required this.initial});

  @override
  State<ColorPickerDialog> createState() => _ColorPickerDialogState();
}

class _ColorPickerDialogState extends State<ColorPickerDialog> {
  late Color _selected;

  @override
  void initState() {
    super.initState();
    _selected = widget.initial;
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;

    return AlertDialog(
      title: const Text('Pick a color'),
      content: SizedBox(
        width: AppTheme.spaceMD * 25,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: AppTheme.iconXXL * AppTheme.iconScale(context),
              height: AppTheme.iconXXL * AppTheme.iconScale(context),
              decoration: BoxDecoration(
                color: _selected,
                borderRadius: BorderRadius.circular(
                  AppTheme.radiusMD * AppTheme.radiusScale(context),
                ),
                border: Border.all(color: colors.outlineVariant),
              ),
            ),
            SizedBox(
              height: AppTheme.spaceLG * AppTheme.spaceScale(context),
            ),
            Wrap(
              spacing: AppTheme.spaceSM * AppTheme.spaceScale(context),
              runSpacing: AppTheme.spaceSM * AppTheme.spaceScale(context),
              children: [
                Colors.redAccent,
                Colors.pinkAccent,
                Colors.purpleAccent,
                Colors.deepPurpleAccent,
                Colors.indigoAccent,
                Colors.blueAccent,
                Colors.lightBlueAccent,
                Colors.cyanAccent,
                Colors.tealAccent,
                Colors.greenAccent,
                Colors.lightGreenAccent,
                Colors.limeAccent,
                Colors.yellowAccent,
                Colors.amberAccent,
                Colors.orangeAccent,
                Colors.deepOrangeAccent,
              ].map((color) {
                return GestureDetector(
                  onTap: () => setState(() => _selected = color),
                  child: Container(
                    width: AppTheme.iconLG * AppTheme.iconScale(context),
                    height: AppTheme.iconLG * AppTheme.iconScale(context),
                    decoration: BoxDecoration(
                      color: color,
                      borderRadius: BorderRadius.circular(
                        AppTheme.radiusSM * AppTheme.radiusScale(context),
                      ),
                      border: _selected == color
                          ? Border.all(
                              color: colors.onSurface,
                              width: 2 * AppTheme.spaceScale(context),
                            )
                          : null,
                    ),
                  ),
                );
              }).toList(),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _selected),
          child: const Text('Select'),
        ),
      ],
    );
  }
}
