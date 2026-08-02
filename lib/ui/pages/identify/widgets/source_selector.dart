import 'package:flutter/material.dart';
import 'package:tawai/src/bindings/bindings.dart';
import 'package:tawai/ui/pages/identify/models/identify_source.dart';
import 'package:tawai/ui/theme/app_theme.dart';

class SourceSelector extends StatelessWidget {
  final IdentifySource selected;
  final List<LibrarySourceInfo> librarySources;
  final ValueChanged<IdentifySource> onChanged;

  const SourceSelector({
    super.key,
    required this.selected,
    required this.librarySources,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Container(
      padding: EdgeInsets.all(AppTheme.spaceMD * AppTheme.spaceScale(context)),
      child: DropdownButtonFormField<IdentifySource>(
        value: selected,
        decoration: InputDecoration(
          labelText: 'Source',
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(AppTheme.radiusMD),
          ),
          contentPadding: EdgeInsets.symmetric(
            horizontal: AppTheme.spaceMD,
            vertical: AppTheme.spaceSM,
          ),
        ),
        items: [
          DropdownMenuItem(
            value: const UnidentifiedSource(),
            child: Row(
              children: [
                Icon(Icons.help_outline, size: AppTheme.iconMD, color: colors.error),
                const SizedBox(width: AppTheme.spaceSM),
                const Text('Unidentified'),
              ],
            ),
          ),
          ...librarySources.map((src) => DropdownMenuItem(
                value: LibrarySource(src),
                child: Row(
                  children: [
                    Icon(Icons.folder, size: AppTheme.iconMD, color: colors.primary),
                    const SizedBox(width: AppTheme.spaceSM),
                    Text(src.name),
                  ],
                ),
              )),
        ],
        onChanged: (v) {
          if (v != null) onChanged(v);
        },
      ),
    );
  }
}
